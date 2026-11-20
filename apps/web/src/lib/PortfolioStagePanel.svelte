<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { saveArtifact } from "./artifacts";
  import { downloadStageArtifact, stageShortId } from "./stage";
  import {
    cancel,
    clearSelection,
    load,
    remove,
    select,
    stages,
    submitPortfolio,
  } from "./stages.svelte";
  import type { ArtifactRef, StageState } from "./types";

  const view = $derived(stages());
  const rows = $derived(view.items.filter((item) => item.kind === "portfolio"));
  let decisionsArtifact = $state("");
  let configId = $state("");
  let submitting = $state(false);
  let downloading = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let saved = $state<string | null>(null);
  let busy = $state(false);

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

  function number(value: number): string {
    return value.toFixed(6);
  }

  async function run(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (submitting) {
      return;
    }
    submitting = true;
    await submitPortfolio(decisionsArtifact.trim(), configId.trim());
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
      saveArtifact(bytes, artifact.name.split("/").at(-1) ?? artifact.name);
      saved = artifact.name;
    } catch (error) {
      downloadError = errorText(error);
    } finally {
      downloading = null;
    }
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Portfolio stage</h2>
    <span class="badge warn">diagnostic · not evidence</span>
    <button onclick={() => void load()} disabled={view.loading}>Refresh</button>
  </div>
  <p class="muted small">
    The stage replays a frozen decisions artifact through execution and portfolio
    accounting. It does not decide a signal, and its P&amp;L is not evidence.
  </p>

  <form class="row" data-testid="portfolio-submit" onsubmit={(event) => void run(event)}>
    <input
      bind:value={decisionsArtifact}
      placeholder="decisions_artifact_id"
      aria-label="Decisions artifact id"
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
      disabled={submitting || decisionsArtifact.trim() === "" || configId.trim() === ""}
    >
      {submitting ? "Submitting…" : "Run portfolio stage"}
    </button>
  </form>

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  {#if view.selected && (view.detail === null || view.detail.kind === "portfolio")}
    <section class="detail stack" data-testid="portfolio-detail">
      <div class="row">
        <button onclick={clearSelection} aria-label="Back to portfolios">Back</button>
        <h3>Portfolio detail</h3>
        {#if view.detail}
          <span class="mono" title={view.detail.stage_run_id}>{stageShortId(view.detail.stage_run_id)}</span>
          <span class="badge {stateBadge(view.detail.state)}">{view.detail.state}</span>
          {#if view.detail.state === "queued" || view.detail.state === "running"}
            <button data-testid="portfolio-cancel" onclick={() => void stop()} disabled={busy}>
              Cancel
            </button>
          {/if}
          <button data-testid="portfolio-delete" onclick={() => void destroy()} disabled={busy}>
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
          Evidence: false. The published P&amp;L is diagnostic accounting, not an accepted result.
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

        <h4>Artifacts</h4>
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
                      data-testid="portfolio-fetch"
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

          {#if view.portfolioLoading}
            <div class="notice">Reading published bundles…</div>
          {:else if view.portfolioError}
            <div class="notice error" role="alert">{view.portfolioError}</div>
          {:else if view.portfolio}
            <h4>Metrics</h4>
            <table data-testid="portfolio-metrics">
              <tbody>
                <tr><th>Initial cash</th><td>{number(view.portfolio.metrics.initial_cash)}</td></tr>
                <tr><th>Final equity</th><td>{number(view.portfolio.metrics.final_equity)}</td></tr>
                <tr><th>Net P&amp;L</th><td>{number(view.portfolio.metrics.net_pnl)}</td></tr>
                <tr><th>Execution fees</th><td>{number(view.portfolio.metrics.execution_fees)}</td></tr>
                <tr><th>Commission</th><td>{number(view.portfolio.metrics.commission)}</td></tr>
                <tr><th>Funding paid</th><td>{number(view.portfolio.metrics.funding_paid)}</td></tr>
                <tr><th>Funding received</th><td>{number(view.portfolio.metrics.funding_received)}</td></tr>
                <tr><th>Max drawdown</th><td>{number(view.portfolio.metrics.max_drawdown)}</td></tr>
                <tr><th>Exposure</th><td>{view.portfolio.metrics.exposure === null ? "—" : number(view.portfolio.metrics.exposure)}</td></tr>
                <tr><th>Halted</th><td>{view.portfolio.metrics.halted}</td></tr>
              </tbody>
            </table>

            <h4>Summary</h4>
            <table data-testid="portfolio-summary">
              <tbody>
                {#each view.portfolio.summary as entry (entry.label)}
                  <tr><th>{entry.label}</th><td>{entry.value}</td></tr>
                {/each}
              </tbody>
            </table>

            <h4>Execution summary</h4>
            <table data-testid="portfolio-execution-summary">
              <tbody>
                {#each view.portfolio.executionSummary as entry (entry.label)}
                  <tr><th>{entry.label}</th><td>{entry.value}</td></tr>
                {/each}
              </tbody>
            </table>

            <h4>Orders</h4>
            <table data-testid="portfolio-orders">
              <thead>
                <tr>{#each view.portfolio.orders.columns as column (column)}<th>{column}</th>{/each}</tr>
              </thead>
              <tbody>
                {#each view.portfolio.orders.rows as row, index (index)}
                  <tr>{#each row as value, cell (cell)}<td class="mono">{value}</td>{/each}</tr>
                {/each}
              </tbody>
            </table>

            <h4>Fills</h4>
            <table data-testid="portfolio-fills">
              <thead>
                <tr>{#each view.portfolio.fills.columns as column (column)}<th>{column}</th>{/each}</tr>
              </thead>
              <tbody>
                {#each view.portfolio.fills.rows as row, index (index)}
                  <tr>{#each row as value, cell (cell)}<td class="mono">{value}</td>{/each}</tr>
                {/each}
              </tbody>
            </table>

            <h4>Rejections</h4>
            <table data-testid="portfolio-rejections">
              <thead>
                <tr>{#each view.portfolio.rejections.columns as column (column)}<th>{column}</th>{/each}</tr>
              </thead>
              <tbody>
                {#each view.portfolio.rejections.rows as row, index (index)}
                  <tr>{#each row as value, cell (cell)}<td class="mono">{value}</td>{/each}</tr>
                {/each}
              </tbody>
            </table>

            <h4>Trades</h4>
            <table>
              <thead>
                <tr>{#each view.portfolio.trades.columns as column (column)}<th>{column}</th>{/each}</tr>
              </thead>
              <tbody>
                {#each view.portfolio.trades.rows as row, index (index)}
                  <tr>{#each row as value, cell (cell)}<td class="mono">{value}</td>{/each}</tr>
                {/each}
              </tbody>
            </table>

            <h4>Risk events</h4>
            <table>
              <thead>
                <tr>{#each view.portfolio.riskEvents.columns as column (column)}<th>{column}</th>{/each}</tr>
              </thead>
              <tbody>
                {#each view.portfolio.riskEvents.rows as row, index (index)}
                  <tr>{#each row as value, cell (cell)}<td class="mono">{value}</td>{/each}</tr>
                {/each}
              </tbody>
            </table>
          {/if}
        {/if}
      {/if}
    </section>
  {:else if view.loading && rows.length === 0}
    <div class="notice">Loading portfolio stages…</div>
  {:else if rows.length === 0}
    <div class="notice">No portfolio stages. Submit one above.</div>
  {:else}
    <table data-testid="portfolio-table" aria-busy={view.loading}>
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
