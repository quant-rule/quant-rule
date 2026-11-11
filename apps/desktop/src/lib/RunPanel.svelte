<script lang="ts">
  import { call } from "./api";
  import { errorMessage } from "./errors";
  import { formatCount, timeframeCode } from "./format";
  import type { ModelRunView, RunSummaryView } from "./types";

  let { runPath }: { runPath: string } = $props();

  let model = $state<ModelRunView | null>(null);
  let busy = $state(false);
  let error = $state("");
  let exportPath = $state("");

  async function run() {
    error = "";
    model = null;
    busy = true;
    try {
      model = await call<ModelRunView>("run_model", { config: runPath });
      exportPath = `${model.run.output_root}/run-summary.json`;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function exportSummary() {
    if (!model) {
      return;
    }
    error = "";
    busy = true;
    try {
      const summary = await call<RunSummaryView>("export_run_summary", {
        config: runPath,
        out: exportPath,
      });
      model = { ...model, run: summary };
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="stack">
  <div class="card">
    <div class="panel-title">
      <div>
        <h2>Model run</h2>
        <p class="muted small mono">{runPath || "Select a run config in Configs."}</p>
      </div>
      <button class="primary" onclick={run} disabled={busy || runPath === ""}>
        {busy ? "Running…" : "Run model"}
      </button>
    </div>

    {#if error}
      <p class="error">{error}</p>
    {:else if model}
      <div class="grid">
        <div class="stat">
          <div class="muted small">Instrument</div>
          <div class="value">{model.run.instrument}</div>
        </div>
        <div class="stat">
          <div class="muted small">Timeframes</div>
          <div class="value">{model.timeframes.length}</div>
        </div>
        <div class="stat">
          <div class="muted small">Events</div>
          <div class="value">{formatCount(model.events)}</div>
        </div>
      </div>

      <table style="margin-top: 1rem">
        <thead>
          <tr><th>Timeframe</th><th>Bars</th><th>Events</th><th>Levels</th></tr>
        </thead>
        <tbody>
          {#each model.timeframes as timeframe (timeframe.timeframe_secs)}
            <tr>
              <td>{timeframeCode(timeframe.timeframe_secs)}</td>
              <td>{formatCount(timeframe.bars)}</td>
              <td>{formatCount(timeframe.events)}</td>
              <td>{formatCount(timeframe.levels)}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="row" style="margin-top: 1rem">
        <input bind:value={exportPath} aria-label="Run summary export path" />
        <button onclick={exportSummary} disabled={busy || exportPath.trim() === ""}>
          Export summary
        </button>
      </div>
    {:else}
      <p class="muted">
        The model computes every configured timeframe once and publishes counts,
        levels, and events. It does not publish a cross-timeframe vote.
      </p>
    {/if}
  </div>
</div>
