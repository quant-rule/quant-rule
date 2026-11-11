<script lang="ts">
  import { call } from "./api";
  import { errorMessage } from "./errors";
  import { formatCount, formatTime } from "./format";
  import type { DatasetRef, DatasetSummaryView } from "./types";

  let { datasets }: { datasets: DatasetRef[] } = $props();

  let selected = $state("");
  let summary = $state<DatasetSummaryView | null>(null);
  let busy = $state(false);
  let error = $state("");

  async function inspect(path: string) {
    selected = path;
    summary = null;
    error = "";
    busy = true;
    try {
      summary = await call<DatasetSummaryView>("dataset_summary", { path });
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="split">
  <div class="card">
    <h2>Datasets</h2>
    <p class="muted small">Local M1 CSV sources in <code>data/sources/</code>.</p>
    <div class="list">
      {#each datasets as dataset (dataset.path)}
        <button
          class:active={selected === dataset.path}
          onclick={() => inspect(dataset.path)}
          disabled={busy}
        >
          <strong>{dataset.name}</strong>
          <div class="muted small mono">{dataset.path}</div>
        </button>
      {:else}
        <div class="muted">No datasets. Open a workspace with CSV sources.</div>
      {/each}
    </div>
  </div>

  <div class="card">
    <h2>Dataset summary</h2>
    {#if busy}
      <p class="muted">Reading the source…</p>
    {:else if error}
      <p class="error">{error}</p>
    {:else if summary}
      <table>
        <tbody>
          <tr><th>Name</th><td>{summary.name}</td></tr>
          <tr><th>Path</th><td class="mono small">{summary.path}</td></tr>
          <tr><th>M1 candles</th><td>{formatCount(summary.candles)}</td></tr>
          <tr><th>First open</th><td>{formatTime(summary.first_open_time)}</td></tr>
          <tr><th>Last open</th><td>{formatTime(summary.last_open_time)}</td></tr>
        </tbody>
      </table>
      <p class="muted small" style="margin-top: 0.8rem">
        The summary reads the same strict source the runner loads. It publishes
        counts and open times, not a price statistic.
      </p>
    {:else}
      <p class="muted">Select a dataset to inspect it.</p>
    {/if}
  </div>
</div>
