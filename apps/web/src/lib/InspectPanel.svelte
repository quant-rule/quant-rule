<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { datasets, load as loadDatasets } from "./datasets.svelte";
  import { inspectCandles, inspectEvents, inspectTimeframes } from "./inspect";
  import { profiles } from "./profiles.svelte";
  import type {
    DatasetSummary,
    InspectEvents,
    TimeframeInspection,
  } from "./types";

  type Mode = "candles" | "timeframes" | "events";

  const data = $derived(datasets());
  const first = $derived(data.items[0]?.data_version ?? "");
  const active = $derived(profiles().active);
  let dataVersion = $state("");
  let mode = $state<Mode>("candles");
  let timeframe = $state("5m");
  let loading = $state(false);
  let error = $state<string | null>(null);
  let candles = $state<DatasetSummary | null>(null);
  let ladder = $state<TimeframeInspection | null>(null);
  let events = $state<InspectEvents | null>(null);
  let scope = $state<string | null>(null);

  onMount(() => {
    void loadDatasets();
  });

  $effect(() => {
    if (dataVersion === "" && first !== "") {
      dataVersion = first;
    }
  });

  // Inspection reads a profile-scoped dataset; a profile switch drops the
  // previous result instead of presenting it under the new profile.
  $effect(() => {
    if (scope === null) {
      scope = active;
      return;
    }
    if (scope !== active) {
      scope = active;
      dataVersion = "";
      candles = null;
      ladder = null;
      events = null;
      error = null;
    }
  });

  function eventNumber(event: Record<string, unknown>, first: string, second: string): string {
    const value = event[first] ?? event[second];
    return typeof value === "number" ? String(value) : "—";
  }

  async function run(): Promise<void> {
    loading = true;
    error = null;
    candles = null;
    ladder = null;
    events = null;
    try {
      if (mode === "candles") {
        candles = await inspectCandles(dataVersion);
      } else if (mode === "timeframes") {
        ladder = await inspectTimeframes(dataVersion);
      } else {
        events = await inspectEvents(dataVersion, timeframe);
      }
    } catch (failure) {
      error = errorText(failure);
    } finally {
      loading = false;
    }
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Inspect</h2>
    <span class="badge">read-only</span>
  </div>
  <p class="muted small">
    Inspect reads the stored bytes and the published event records. It never
    starts a run and never changes a dataset.
  </p>

  <form
    class="controls"
    onsubmit={(event) => {
      event.preventDefault();
      void run();
    }}
  >
    <label>
      <span class="muted small">Data version</span>
      <input
        bind:value={dataVersion}
        placeholder="sha256:…"
        spellcheck="false"
        aria-label="Data version"
      />
    </label>
    <label>
      <span class="muted small">Inspection</span>
      <select bind:value={mode} aria-label="Inspection">
        <option value="candles">Candles</option>
        <option value="timeframes">Timeframes</option>
        <option value="events">Events</option>
      </select>
    </label>
    {#if mode === "events"}
      <label>
        <span class="muted small">Timeframe</span>
        <input bind:value={timeframe} aria-label="Timeframe" />
      </label>
    {/if}
    <button type="submit" disabled={loading || dataVersion === ""}>
      {loading ? "Inspecting…" : "Inspect"}
    </button>
  </form>

  {#if error}
    <div class="notice error" role="alert">{error}</div>
  {/if}

  {#if candles}
    <h3>Candles</h3>
    <table>
      <tbody>
        <tr><th>Data version</th><td class="mono">{candles.data_version}</td></tr>
        <tr><th>Name</th><td>{candles.name}</td></tr>
        <tr><th>Candles</th><td>{candles.candles}</td></tr>
        <tr><th>First bar</th><td class="mono">{candles.first_bar}</td></tr>
        <tr><th>Last bar</th><td class="mono">{candles.last_bar}</td></tr>
        <tr><th>Bytes</th><td>{candles.bytes}</td></tr>
      </tbody>
    </table>
  {:else if ladder}
    <h3>Timeframes</h3>
    <table>
      <thead>
        <tr>
          <th>Timeframe</th>
          <th>Bars</th>
          <th>Partial buckets</th>
          <th>First bar</th>
          <th>Last bar</th>
        </tr>
      </thead>
      <tbody>
        {#each ladder.timeframes as row (row.timeframe)}
          <tr>
            <td class="mono">{row.timeframe}</td>
            <td>{row.bars}</td>
            <td>{row.partial_buckets}</td>
            <td class="mono">{row.first_bar ?? "—"}</td>
            <td class="mono">{row.last_bar ?? "—"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if events}
    <h3>Events — {events.timeframe}</h3>
    {#if events.events.length === 0}
      <div class="notice">No events were published for this timeframe.</div>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Type</th>
            <th>Source</th>
            <th>Occurred bar</th>
            <th>Available bar</th>
          </tr>
        </thead>
        <tbody>
          {#each events.events as event, index (index)}
            <tr>
              <td>{eventNumber(event, "event_type", "type")}</td>
              <td>{eventNumber(event, "source", "source")}</td>
              <td>{eventNumber(event, "occurred_bar", "bar")}</td>
              <td>{eventNumber(event, "available_bar", "available_bar")}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</section>

<style>
  section {
    max-width: 80rem;
    gap: 0.8rem;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 0.6rem;
  }

  label {
    display: grid;
    gap: 0.25rem;
    min-width: 12rem;
  }

  input,
  select {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.4rem 0.55rem;
  }
</style>
