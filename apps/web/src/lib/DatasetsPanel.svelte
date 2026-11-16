<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { datasetRecord } from "./datasets";
  import { datasets, load, upload } from "./datasets.svelte";
  import type { DatasetRecord } from "./types";

  const view = $derived(datasets());
  let file = $state<File | null>(null);
  let input = $state<HTMLInputElement | null>(null);
  let tolerant = $state(false);
  let selected = $state<DatasetRecord | null>(null);
  let detailLoading = $state(false);
  let detailError = $state<string | null>(null);

  onMount(() => {
    void load();
  });

  $effect(() => {
    if (selected && !view.items.some((item) => item.data_version === selected?.summary.data_version)) {
      selected = null;
    }
  });

  async function open(dataVersion: string): Promise<void> {
    detailLoading = true;
    detailError = null;
    try {
      selected = await datasetRecord(dataVersion);
    } catch (error) {
      detailError = errorText(error);
      selected = null;
    } finally {
      detailLoading = false;
    }
  }

  function closeDetail(): void {
    selected = null;
  }

  function choose(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    file = target.files?.[0] ?? null;
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!file || view.uploading) {
      return;
    }
    await upload(file, tolerant);
    file = null;
    if (input) {
      input.value = "";
    }
  }

  function utc(seconds: number): string {
    return new Date(seconds * 1000)
      .toISOString()
      .replace("T", " ")
      .replace(".000Z", "Z");
  }

  function version(value: string): string {
    return value.length > 20 ? `${value.slice(0, 17)}…` : value;
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Datasets</h2>
    <span class="badge">{view.items.length} total</span>
    <button onclick={() => void load()} disabled={view.loading}>Refresh</button>
  </div>

  <form class="row" data-testid="dataset-upload" onsubmit={(event) => void submit(event)}>
    <input
      bind:this={input}
      type="file"
      accept=".csv,text/csv"
      aria-label="CSV file"
      onchange={choose}
    />
    <label class="row small">
      <input type="checkbox" bind:checked={tolerant} />
      Skip bad rows
    </label>
    <button type="submit" disabled={!file || view.uploading}>
      {view.uploading ? "Uploading…" : "Upload"}
    </button>
  </form>

  {#if view.uploading}
    <div class="progress" role="progressbar" aria-label="Uploading"><span></span></div>
  {/if}

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  {#if view.lastQuality}
    <div class="notice" role="status">
      {#if view.lastQuality.rejected > 0}
        {view.lastQuality.rejected} of {view.lastQuality.rows_read} rows skipped.
        First: line {view.lastQuality.skipped[0].line}
        ({view.lastQuality.skipped[0].reason}).
      {:else}
        All {view.lastQuality.rows_read} rows accepted.
      {/if}
    </div>
  {/if}

  {#if view.loading && view.items.length === 0}
    <div class="notice">Loading datasets…</div>
  {:else if view.items.length === 0}
    <div class="notice">No datasets. Upload a CSV.</div>
  {:else}
    <table data-testid="dataset-table">
      <thead>
        <tr>
          <th>Version</th>
          <th>Name</th>
          <th>Candles</th>
          <th>First bar (UTC)</th>
          <th>Last bar (UTC)</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each view.items as item (item.data_version)}
          <tr>
            <td class="mono" title={item.data_version}>{version(item.data_version)}</td>
            <td>{item.name}</td>
            <td>{item.candles}</td>
            <td class="mono">{utc(item.first_bar)}</td>
            <td class="mono">{utc(item.last_bar)}</td>
            <td>
              <button
                data-testid="dataset-detail-open"
                aria-label="Details"
                title={`Details for ${item.name}`}
                onclick={() => void open(item.data_version)}
                disabled={detailLoading}
              >
                Details
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    {#if detailLoading}
      <div class="notice" role="status">Loading dataset detail…</div>
    {/if}
    {#if detailError}
      <div class="notice error" role="alert">{detailError}</div>
    {/if}
    {#if selected}
      <section class="detail" data-testid="dataset-detail">
        <div class="row">
          <h3>Dataset detail</h3>
          <button onclick={closeDetail}>Close</button>
        </div>
        <table>
          <tbody>
            <tr><th>Name</th><td>{selected.summary.name}</td></tr>
            <tr><th>Data version</th><td class="mono">{selected.summary.data_version}</td></tr>
            <tr><th>Object key</th><td class="mono">{selected.object_key}</td></tr>
            <tr><th>SHA-256</th><td class="mono">{selected.sha256}</td></tr>
            <tr><th>Candles</th><td>{selected.summary.candles}</td></tr>
            <tr>
              <th>Coverage</th>
              <td class="mono">{utc(selected.summary.first_bar)} → {utc(selected.summary.last_bar)}</td>
            </tr>
            <tr><th>Bytes</th><td>{selected.summary.bytes}</td></tr>
            <tr><th>Created</th><td class="mono">{selected.created_at}</td></tr>
            <tr>
              <th>Quality</th>
              <td>
                {#if selected.quality}
                  {selected.quality.accepted}/{selected.quality.rows_read} accepted,
                  {selected.quality.rejected} rejected
                {:else}
                  none
                {/if}
              </td>
            </tr>
          </tbody>
        </table>
      </section>
    {/if}
  {/if}
</section>

<style>
  section {
    max-width: 70rem;
    gap: 0.8rem;
  }

  form {
    flex-wrap: wrap;
  }

  .progress {
    height: 4px;
    background: var(--line);
    border-radius: 999px;
    overflow: hidden;
  }

  .progress span {
    display: block;
    width: 35%;
    height: 100%;
    background: var(--accent);
    animation: slide 1s linear infinite;
  }

  @keyframes slide {
    from { transform: translateX(-100%); }
    to { transform: translateX(300%); }
  }

  @media (prefers-reduced-motion: reduce) {
    .progress span { animation: none; width: 100%; }
  }
</style>
