<script lang="ts">
  import { onMount } from "svelte";
  import { datasets, load, upload } from "./datasets.svelte";

  const view = $derived(datasets());
  let file = $state<File | null>(null);
  let input = $state<HTMLInputElement | null>(null);
  let tolerant = $state(false);

  onMount(() => {
    void load();
  });

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

  <form class="row" onsubmit={(event) => void submit(event)}>
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
    <table>
      <thead>
        <tr>
          <th>Version</th>
          <th>Name</th>
          <th>Candles</th>
          <th>First bar (UTC)</th>
          <th>Last bar (UTC)</th>
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
          </tr>
        {/each}
      </tbody>
    </table>
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
