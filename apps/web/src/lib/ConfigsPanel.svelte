<script lang="ts">
  import { onMount } from "svelte";
  import { canSubmit } from "./configs";
  import { configs, load, submit } from "./configs.svelte";
  import { datasets, load as loadDatasets } from "./datasets.svelte";
  import { shortId } from "./runs";

  let { onOpenRuns }: { onOpenRuns?: () => void } = $props();

  const view = $derived(configs());
  const data = $derived(datasets());
  let dataVersion = $state("");

  onMount(() => {
    void (async () => {
      await loadDatasets();
      const items = datasets().items;
      if (dataVersion === "" && items.length > 0) {
        dataVersion = items[0].data_version;
      }
      await load();
    })();
  });
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
    <table>
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
            <td>
              <button
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
