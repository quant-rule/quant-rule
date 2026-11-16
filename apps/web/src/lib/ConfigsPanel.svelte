<script lang="ts">
  import { onMount } from "svelte";
  import { canSubmit, findConfig } from "./configs";
  import { configs, load, submit } from "./configs.svelte";
  import { datasets, load as loadDatasets } from "./datasets.svelte";
  import { shortId } from "./runs";
  import type { ConfigSummary } from "./types";

  let { onOpenRuns }: { onOpenRuns?: () => void } = $props();

  const view = $derived(configs());
  const data = $derived(datasets());
  let dataVersion = $state("");
  let selected = $state<ConfigSummary | null>(null);

  onMount(() => {
    void (async () => {
      await loadDatasets();
      await load();
    })();
  });

  // A profile switch replaces the dataset list; the old selection must not
  // survive as the dataset a submitted run would name.
  $effect(() => {
    if (!data.items.some((item) => item.data_version === dataVersion)) {
      dataVersion = data.items[0]?.data_version ?? "";
    }
  });

  function open(configId: string): void {
    selected = findConfig(view.items, configId);
  }

  function closeDetail(): void {
    selected = null;
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Configs</h2>
    <span class="badge">{view.items.length} total</span>
    <button onclick={() => void load()} disabled={view.loading}>Refresh</button>
  </div>

  <label class="row">
    <span class="muted small">Dataset</span>
    <select bind:value={dataVersion} aria-label="Dataset">
      {#if data.items.length === 0}
        <option value="">No datasets — upload one</option>
      {/if}
      {#each data.items as item (item.data_version)}
        <option value={item.data_version}>{item.name} · {shortId(item.data_version)}</option>
      {/each}
    </select>
  </label>

  {#if data.items.length === 0}
    <div class="notice">Upload a dataset before submitting.</div>
  {/if}

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  {#if view.lastRun}
    <div class="notice row">
      <span>Run queued: <span class="mono">{shortId(view.lastRun)}</span></span>
      {#if onOpenRuns}
        <button onclick={onOpenRuns}>View runs</button>
      {/if}
    </div>
  {/if}

  {#if view.loading && view.items.length === 0}
    <div class="notice">Loading configs…</div>
  {:else if view.items.length === 0}
    <div class="notice">No configs on the server.</div>
  {:else}
    <table data-testid="config-table">
      <thead>
        <tr>
          <th>Config</th>
          <th>Origin</th>
          <th>Validity</th>
          <th>Parser message</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each view.items as item (item.config_id)}
          <tr>
            <td class="mono">{item.name}</td>
            <td><span class="badge">{item.origin}</span></td>
            <td>
              <span class="badge {item.valid ? 'ok' : 'error'}">
                {item.valid ? "valid" : "invalid"}
              </span>
            </td>
            <td class="message">{item.message ?? ""}</td>
            <td class="actions">
              <button
                data-testid="config-detail-open"
                aria-label="Details"
                title={`Details for ${item.name}`}
                onclick={() => open(item.config_id)}
              >
                Details
              </button>
              <button
                data-testid="run-submit"
                onclick={() => void submit(item.config_id, dataVersion)}
                disabled={!canSubmit(item, dataVersion, view.submitting)}
                aria-label={`Submit ${item.name}`}
                title={item.valid ? "Submit this config" : (item.message ?? "invalid config")}
              >
                {view.submitting === item.config_id ? "Submitting…" : "Submit"}
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    {#if selected}
      <section class="detail" data-testid="config-detail">
        <div class="row">
          <h3>Config detail</h3>
          <button onclick={closeDetail}>Close</button>
        </div>
        <table>
          <tbody>
            <tr><th>Config id</th><td class="mono">{selected.config_id}</td></tr>
            <tr><th>Name</th><td>{selected.name}</td></tr>
            <tr><th>Origin</th><td>{selected.origin}</td></tr>
            <tr><th>Valid</th><td>{selected.valid ? "yes" : "no"}</td></tr>
            <tr><th>Config hash</th><td class="mono">{selected.config_hash ?? "none"}</td></tr>
            <tr><th>Parser message</th><td class="message">{selected.message ?? "none"}</td></tr>
          </tbody>
        </table>
      </section>
    {/if}
  {/if}
</section>

<style>
  section {
    max-width: 80rem;
    gap: 0.8rem;
  }

  select {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.35rem 0.6rem;
    max-width: 32rem;
  }

  .message {
    max-width: 34rem;
    white-space: pre-wrap;
    font-size: 0.82rem;
  }
</style>
