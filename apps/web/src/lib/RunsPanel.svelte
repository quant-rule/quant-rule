<script lang="ts">
  import { onMount } from "svelte";
  import { connection } from "./connection.svelte";
  import { cancel, load, runs, startPolling } from "./runs.svelte";
  import { elapsed, isActive, shortId, submitted } from "./runs";
  import type { RunState } from "./types";

  const view = $derived(runs());
  const link = $derived(connection());
  let cancelling = $state<string | null>(null);

  onMount(() => startPolling());

  function badge(status: RunState): string {
    switch (status) {
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

  async function stop(runId: string): Promise<void> {
    cancelling = runId;
    await cancel(runId);
    cancelling = null;
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Runs</h2>
    <span class="badge">{view.items.length} total</span>
    <button onclick={() => void load()} disabled={view.loading}>Refresh</button>
  </div>

  {#if link.state === "unreachable"}
    <div class="notice error">Disconnected. The run list is not shown.</div>
  {:else if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  {#if link.state !== "unreachable"}
    {#if view.loading && view.items.length === 0}
      <div class="notice">Loading runs…</div>
    {:else if view.items.length === 0}
      <div class="notice">No runs. Submit one from Configs.</div>
    {:else}
      <table aria-busy={view.loading}>
        <thead>
          <tr>
            <th>Run</th>
            <th>Status</th>
            <th>Config</th>
            <th>Data</th>
            <th>Submitted (UTC)</th>
            <th>Duration</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each view.items as item (item.run_id)}
            <tr>
              <td class="mono" title={item.run_id}>{shortId(item.run_id)}</td>
              <td><span class="badge {badge(item.status)}">{item.status}</span></td>
              <td class="mono">{item.config_id}</td>
              <td class="mono" title={item.data_version}>{shortId(item.data_version)}</td>
              <td class="mono">{submitted(item.submitted_at)}</td>
              <td>{elapsed(item.elapsed_ms)}</td>
              <td>
                {#if isActive(item.status)}
                  <button
                    onclick={() => void stop(item.run_id)}
                    disabled={cancelling === item.run_id}
                    aria-label={`Cancel ${item.run_id}`}
                  >
                    Cancel
                  </button>
                {/if}
              </td>
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
</style>
