<script lang="ts">
  import { connection, refresh } from "./connection.svelte";

  const state = $derived(connection());

  const label = $derived(
    state.state === "connected"
      ? "connected"
      : state.state === "degraded"
        ? "degraded"
        : state.state === "mismatch"
          ? "version mismatch"
          : state.state === "unreachable"
            ? "unreachable"
            : "unknown",
  );

  const klass = $derived(
    state.state === "connected"
      ? "ok"
      : state.state === "degraded"
        ? "warn"
        : state.state === "mismatch"
          ? "error"
          : state.state === "unreachable"
            ? "error"
            : "",
  );
</script>

<section class="stack">
  <div class="row">
    <h2>Connection</h2>
    <span class="badge {klass}">{label}</span>
    <button onclick={refresh}>Refresh</button>
  </div>

  {#if state.error}
    <div class="notice error">{state.error}</div>
  {/if}

  {#if state.version}
    <table>
      <tbody>
        <tr><th>Product</th><td class="mono">{state.version.product}</td></tr>
        <tr><th>API</th><td class="mono">{state.version.api}</td></tr>
        <tr><th>Image digest</th><td class="mono">{state.version.image_digest ?? "unknown"}</td></tr>
      </tbody>
    </table>
  {/if}

  {#if state.health}
    <table>
      <thead>
        <tr><th>Service</th><th>State</th><th>Detail</th></tr>
      </thead>
      <tbody>
        {#each state.health.services as service (service.name)}
          <tr>
            <td class="mono">{service.name}</td>
            <td>{service.state}</td>
            <td class="muted">{service.detail ?? ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="muted small">
      Queue depth {state.health.queue_depth}, workers {state.health.workers}.
    </p>
  {/if}
</section>

<style>
  section { max-width: 60rem; }
</style>
