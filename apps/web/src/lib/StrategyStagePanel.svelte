<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { saveArtifact } from "./artifacts";
  import { downloadStageArtifact, stageShortId } from "./stage";
  import {
    cancel,
    clearSelection,
    explain,
    load,
    remove,
    select,
    stages,
    submitStrategy,
  } from "./stages.svelte";
  import type { ArtifactRef, StageState } from "./types";

  const view = $derived(stages());
  const rows = $derived(view.items.filter((item) => item.kind === "strategy"));
  let modelArtifact = $state("");
  let configId = $state("cfg_abc");
  let submitting = $state(false);
  let downloading = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let saved = $state<string | null>(null);
  let busy = $state(false);
  let bar = $state(0);

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
    await submitStrategy(modelArtifact.trim(), configId.trim());
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

  async function read(artifact: ArtifactRef): Promise<void> {
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

  async function explainBar(): Promise<void> {
    if (view.detail === null) {
      return;
    }
    await explain(view.detail.stage_run_id, bar);
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Strategy stage</h2>
    <span class="badge warn">diagnostic · not evidence</span>
    <button onclick={() => void load()} disabled={view.loading}>Refresh</button>
  </div>
  <p class="muted small">
    The standalone strategy simulates its own position. It never reads execution
    feedback, fills, cash, P&amp;L, exposure, or drawdown.
  </p>

  <form class="row" data-testid="strategy-submit" onsubmit={(event) => void run(event)}>
    <input
      bind:value={modelArtifact}
      placeholder="model_artifact_id"
      aria-label="Model artifact id"
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
    <button
      type="submit"
      disabled={submitting || modelArtifact.trim() === "" || configId.trim() === ""}
    >
      {submitting ? "Submitting…" : "Run strategy stage"}
    </button>
  </form>

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  {#if view.selected && (view.detail === null || view.detail.kind === "strategy")}
    <section class="detail stack" data-testid="strategy-detail">
      <div class="row">
        <button onclick={clearSelection} aria-label="Back to strategies">Back</button>
        <h3>Strategy detail</h3>
        {#if view.detail}
          <span class="mono" title={view.detail.stage_run_id}>{stageShortId(view.detail.stage_run_id)}</span>
          <span class="badge {stateBadge(view.detail.state)}">{view.detail.state}</span>
          {#if view.detail.state === "queued" || view.detail.state === "running"}
            <button data-testid="strategy-cancel" onclick={() => void stop()} disabled={busy}>
              Cancel
            </button>
          {/if}
          <button data-testid="strategy-delete" onclick={() => void destroy()} disabled={busy}>
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
          Evidence: false. No P&amp;L and no execution feedback.
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

        <h4>Decision bundle</h4>
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
                      data-testid="strategy-fetch"
                      onclick={() => void read(artifact)}
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

          <h4>Published decisions</h4>
          {#if view.decisionsLoading}
            <div class="notice">Reading decisions…</div>
          {:else if view.decisionsError}
            <div class="notice error" role="alert">{view.decisionsError}</div>
          {:else if view.decisions.length === 0}
            <div class="notice">No decision records.</div>
          {:else}
            <table data-testid="strategy-decisions">
              <thead>
                <tr>
                  <th>Bar</th><th>Decision</th><th>Side</th><th>Reason</th>
                  <th>Before</th><th>After</th><th>ATR 14</th>
                </tr>
              </thead>
              <tbody>
                {#each view.decisions as decision (decision.bar_index)}
                  <tr>
                    <td class="mono">{decision.bar_index}</td>
                    <td>{decision.decision}</td>
                    <td>{decision.side ?? "none"}</td>
                    <td>{decision.reason ?? "none"}</td>
                    <td>{decision.position_before}</td>
                    <td>{decision.position_after}</td>
                    <td>{decision.atr14 ?? "none"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}

          <h4>Explain one bar</h4>
          <div class="row">
            <input bind:value={bar} type="number" min="0" aria-label="Decision bar" class="mono" />
            <button data-testid="strategy-explain" onclick={() => void explainBar()} disabled={view.explainLoading}>
              {view.explainLoading ? "Reading…" : "Explain bar"}
            </button>
          </div>
          {#if view.explainError}
            <div class="notice error" role="alert">{view.explainError}</div>
          {/if}
          {#if view.explain}
            <table>
              <tbody>
                <tr><th>Bar</th><td>{view.explain.bar_index}</td></tr>
                <tr><th>Decision</th><td>{view.explain.decision}</td></tr>
                <tr><th>Side</th><td>{view.explain.side ?? "none"}</td></tr>
                <tr><th>Reason</th><td>{view.explain.reason ?? "none"}</td></tr>
                <tr><th>Position before</th><td>{view.explain.position_before}</td></tr>
                <tr><th>Position after</th><td>{view.explain.position_after}</td></tr>
                <tr><th>ATR 14</th><td>{view.explain.atr14 ?? "none"}</td></tr>
              </tbody>
            </table>
          {/if}
        {/if}
      {/if}
    </section>
  {:else if view.loading && rows.length === 0}
    <div class="notice">Loading strategy stages…</div>
  {:else if rows.length === 0}
    <div class="notice">No strategy stages. Submit one above.</div>
  {:else}
    <table data-testid="strategy-table" aria-busy={view.loading}>
      <thead>
        <tr>
          <th>Stage</th><th>State</th><th>Config</th><th>Submitted (UTC)</th><th>Artifact</th>
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
  .link {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--accent);
    font-family: var(--mono);
    cursor: pointer;
  }
</style>
