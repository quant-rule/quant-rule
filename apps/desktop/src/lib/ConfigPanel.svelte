<script lang="ts">
  import { call } from "./api";
  import { errorMessage } from "./errors";
  import type { RunSummaryView } from "./types";

  let {
    runs,
    onSelect,
  }: {
    runs: string[];
    onSelect: (path: string) => void;
  } = $props();

  let checked = $state("");
  let summary = $state<RunSummaryView | null>(null);
  let busy = $state(false);
  let error = $state("");

  async function check(path: string) {
    checked = path;
    error = "";
    summary = null;
    busy = true;
    try {
      summary = await call<RunSummaryView>("check_run", { config: path });
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="split">
  <div class="card">
    <h2>Run configs</h2>
    <p class="muted small">
      The loader validates data, model, execution, and portfolio composition
      without reading a candle.
    </p>
    <div class="list">
      {#each runs as run (run)}
        <button
          class:active={checked === run}
          onclick={() => check(run)}
          disabled={busy}
        >
          <div class="mono small">{run}</div>
        </button>
      {:else}
        <div class="muted">No run configs. Open a workspace with config/.</div>
      {/each}
    </div>
  </div>

  <div class="card">
    <h2>Validation</h2>
    {#if busy}
      <p class="muted">Validating…</p>
    {:else if error}
      <p class="error">{error}</p>
    {:else if summary}
      <table>
        <tbody>
          <tr><th>Name</th><td>{summary.name}</td></tr>
          <tr><th>Instrument</th><td>{summary.instrument}</td></tr>
          <tr><th>Data version</th><td class="mono small">{summary.data_version}</td></tr>
          <tr><th>Data ladder</th><td>{summary.data_timeframes.join(", ")}</td></tr>
          <tr><th>Model ladder</th><td>{summary.model_timeframes.join(", ")}</td></tr>
          <tr><th>Execution</th><td>{summary.execution_fill_timeframe}</td></tr>
          <tr><th>Model</th><td>{summary.model_id ?? "—"}</td></tr>
          <tr><th>Portfolio</th><td>{summary.portfolio_id ?? "—"}</td></tr>
          <tr><th>Initial cash</th><td>{summary.initial_cash}</td></tr>
          <tr><th>Leverage</th><td>{summary.leverage}</td></tr>
          <tr><th>Max positions</th><td>{summary.max_open_positions}</td></tr>
          <tr><th>Config hash</th><td class="mono small">{summary.config_hash}</td></tr>
        </tbody>
      </table>
      <div class="row" style="margin-top: 0.8rem">
        <button class="primary" onclick={() => onSelect(checked)}>
          Use in Run
        </button>
      </div>
    {:else}
      <p class="muted">Select a run config to validate it.</p>
    {/if}
  </div>
</div>
