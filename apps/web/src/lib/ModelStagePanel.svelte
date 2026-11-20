<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { downloadStageArtifact, stageShortId } from "./stage";
  import { saveArtifact } from "./artifacts";
  import {
    cancel,
    clearSelection,
    load,
    loadEvents,
    remove,
    select,
    stages,
    submit,
  } from "./stages.svelte";
  import type { ArtifactRef, StageState } from "./types";

  const view = $derived(stages());
  const rows = $derived(view.items.filter((item) => item.kind === "model"));
  let dataVersion = $state("");
  let configId = $state("");
  let submitting = $state(false);
  let downloading = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let saved = $state<string | null>(null);
  let busy = $state(false);
  let timeframe = $state("5m");

  onMount(() => {
    void load();
  });

  function stateBadge(state: StageState): string {
    switch (state) {
      case "queued":
        return "warn";
      case "running":
        return "ok";
      case "succeeded":
        return "ok";
      case "failed":
        return "error";
      case "cancelled":
        return "";
    }
  }

  async function run(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (submitting) {
      return;
    }
    submitting = true;
    await submit(dataVersion.trim(), configId.trim());
    submitting = false;
  }

  async function stop(): Promise<void> {
    if (view.detail === null || busy) {
      return;
    }
    busy = true;
    await cancel(view.detail.stage_run_id);
    busy = false;
  }

  async function destroy(): Promise<void> {
    if (view.detail === null || busy) {
      return;
    }
    busy = true;
    await remove(view.detail.stage_run_id);
    busy = false;
  }

  async function readEvents(): Promise<void> {
    await loadEvents(timeframe.trim());
  }

  async function download(artifact: ArtifactRef): Promise<void> {
    if (view.detail === null) {
      return;
    }
    downloading = artifact.name;
    downloadError = null;
    saved = null;
    try {
      const bytes = await downloadStageArtifact(view.detail.stage_run_id, artifact);
      saveArtifact(bytes, artifact.name);
      saved = artifact.name;
    } catch (error) {
      downloadError = errorText(error);
    } finally {
      downloading = null;
    }
  }

  function coverageShare(value: number | null): string {
    return value === null ? "—" : value.toFixed(4);
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Model stage</h2>
    <span class="badge">{rows.length} total</span>
    <button onclick={() => void load()} disabled={view.loading}>Refresh</button>
  </div>

  <form class="row" data-testid="stage-submit" onsubmit={(event) => void run(event)}>
    <input
      bind:value={dataVersion}
      placeholder="data_version"
      aria-label="Data version"
      spellcheck="false"
      class="mono"
    />
    <input
      bind:value={configId}
      placeholder="config_id"
      aria-label="Config id"
      spellcheck="false"
      class="mono"
    />
    <button type="submit" disabled={submitting || dataVersion.trim() === "" || configId.trim() === ""}>
      {submitting ? "Submitting…" : "Run model stage"}
    </button>
  </form>

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  {#if view.selected && (view.detail === null || view.detail.kind === "model")}
    <section class="detail stack" data-testid="stage-detail">
      <div class="row">
        <button onclick={clearSelection} aria-label="Back to stages">Back</button>
        <h3>Stage detail</h3>
        {#if view.detail}
          <span class="mono" title={view.detail.stage_run_id}>{stageShortId(view.detail.stage_run_id)}</span>
          <span class="badge {stateBadge(view.detail.state)}">{view.detail.state}</span>
          {#if view.detail.state === "queued" || view.detail.state === "running"}
            <button data-testid="stage-cancel" onclick={() => void stop()} disabled={busy}>
              Cancel
            </button>
          {/if}
          <button data-testid="stage-delete" onclick={() => void destroy()} disabled={busy}>
            Delete
          </button>
        {/if}
      </div>

      {#if view.detailLoading && !view.detail}
        <div class="notice">Loading stage…</div>
      {:else if !view.detail}
        <div class="notice">The stage is unavailable.</div>
      {:else}
        <p class="muted small">
          Owner {view.detail.owner}, config {view.detail.config_id}, attempts {view.detail.attempts}.
          Evidence: false.
        </p>
        {#if view.detail.error}
          <div class="notice error" role="alert">{view.detail.error}</div>
        {/if}

        <h4>Provenance</h4>
        <table>
          <tbody>
            <tr><th>Data version</th><td class="mono">{view.detail.provenance.data_version}</td></tr>
            <tr><th>Config hash</th><td class="mono">{view.detail.provenance.config_hash}</td></tr>
            <tr><th>Model version</th><td class="mono">{view.detail.provenance.model_version}</td></tr>
            <tr><th>Image digest</th><td class="mono">{view.detail.provenance.image_digest}</td></tr>
          </tbody>
        </table>

        <h4>Artifact</h4>
        {#if view.detail.artifact === null}
          <div class="notice">No artifact yet.</div>
        {:else}
          <p class="mono small">artifact_id: {view.detail.artifact.artifact_id}</p>
          <table>
            <thead>
              <tr><th>Name</th><th>Bytes</th><th>SHA-256</th><th>Download</th></tr>
            </thead>
            <tbody>
              {#each view.detail.artifact.files as artifact (artifact.name)}
                <tr>
                  <td class="mono">{artifact.name}</td>
                  <td>{artifact.bytes}</td>
                  <td class="mono" title={artifact.sha256}>{stageShortId(artifact.sha256)}</td>
                  <td>
                    <button
                      data-testid="stage-fetch"
                      onclick={() => void download(artifact)}
                      disabled={downloading !== null}
                      aria-label={`Download ${artifact.name}`}
                    >
                      {downloading === artifact.name ? "Checking…" : "Download"}
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if saved}
            <div class="notice small">{saved} matched the stage record and was saved.</div>
          {/if}
          {#if downloadError}
            <div class="notice error" role="alert">{downloadError}</div>
          {/if}
        {/if}

        <h4>Events and coverage</h4>
        <div class="row">
          <input
            bind:value={timeframe}
            aria-label="Event timeframe"
            class="mono"
            placeholder="5m"
          />
          <button data-testid="stage-events" onclick={() => void readEvents()} disabled={view.eventsLoading}>
            {view.eventsLoading ? "Reading…" : "Read events"}
          </button>
        </div>
        {#if view.eventsError}
          <div class="notice error" role="alert">{view.eventsError}</div>
        {/if}
        {#if view.events}
          <p class="muted small">
            {view.events.events.length} event(s) at {view.events.timeframe}.
          </p>
          <div class="scroll">
            <table>
              <thead>
                <tr>
                  {#each view.events.columns as column (column)}<th>{column}</th>{/each}
                </tr>
              </thead>
              <tbody>
                {#each view.events.events as event, index (index)}
                  <tr>
                    {#each view.events.columns as column (column)}<td class="mono">{event[column]}</td>{/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <h4>Coverage</h4>
          <table>
            <thead>
              <tr><th>Timeframe (s)</th><th>Bars</th><th>Expected</th><th>Coverage</th></tr>
            </thead>
            <tbody>
              {#each view.events.coverage as row (row.timeframe_secs)}
                <tr>
                  <td class="mono">{row.timeframe_secs}</td>
                  <td>{row.bars ?? "—"}</td>
                  <td>{row.expected_bars ?? "—"}</td>
                  <td>{coverageShare(row.coverage_share)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {/if}
    </section>
  {:else if view.loading && rows.length === 0}
    <div class="notice">Loading stages…</div>
  {:else if rows.length === 0}
    <div class="notice">No stage runs. Submit a model stage above.</div>
  {:else}
    <table data-testid="stage-table" aria-busy={view.loading}>
      <thead>
        <tr>
          <th>Stage</th><th>Kind</th><th>State</th><th>Config</th><th>Submitted (UTC)</th><th>Artifact</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as item (item.stage_run_id)}
          <tr>
            <td>
              <button class="link" onclick={() => void select(item.stage_run_id)} aria-label={`Open ${item.stage_run_id}`}>
                {stageShortId(item.stage_run_id)}
              </button>
            </td>
            <td>{item.kind}</td>
            <td><span class="badge {stateBadge(item.state)}">{item.state}</span></td>
            <td class="mono">{item.config_id}</td>
            <td class="mono">{item.submitted_at.replace("T", " ")}</td>
            <td class="mono" title={item.artifact_id ?? "none"}>{item.artifact_id === null ? "none" : stageShortId(item.artifact_id)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  section { max-width: 80rem; gap: 0.8rem; }
  form { flex-wrap: wrap; }
  table { width: 100%; }
  .scroll { overflow-x: auto; }
  .link {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--accent);
    font-family: var(--mono);
    cursor: pointer;
  }
</style>
