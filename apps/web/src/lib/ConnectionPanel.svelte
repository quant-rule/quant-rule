<script lang="ts">
  import { connection, forgetToken, refresh, useToken } from "./connection.svelte";

  const view = $derived(connection());

  let token = $state("");
  let saving = $state(false);

  const label = $derived(
    view.state === "connected"
      ? "connected"
      : view.state === "degraded"
        ? "degraded"
        : view.state === "mismatch"
          ? "version mismatch"
          : view.state === "unreachable"
            ? "unreachable"
            : view.state === "unauthorized"
              ? "token required"
              : "unknown",
  );

  const klass = $derived(
    view.state === "connected"
      ? "ok"
      : view.state === "degraded"
        ? "warn"
        : view.state === "mismatch" ||
            view.state === "unreachable" ||
            view.state === "unauthorized"
          ? "error"
          : "",
  );

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (token.trim() === "" || saving) {
      return;
    }
    saving = true;
    const accepted = await useToken(token);
    saving = false;
    if (accepted) {
      token = "";
    }
  }

  async function forget(): Promise<void> {
    saving = true;
    await forgetToken();
    saving = false;
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Connection</h2>
    <span class="badge {klass}">{label}</span>
    <button onclick={refresh} disabled={saving}>Refresh</button>
  </div>

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  <form class="row" onsubmit={(event) => void save(event)}>
    <input
      type="password"
      bind:value={token}
      placeholder={view.tokenStored ? "Token stored; enter a new one" : "Bearer token"}
      aria-label="Gateway bearer token"
      autocomplete="off"
      spellcheck="false"
    />
    <button type="submit" disabled={token.trim() === "" || saving}>Use token</button>
    {#if view.tokenStored}
      <button type="button" onclick={() => void forget()} disabled={saving}>
        Forget token
      </button>
    {/if}
  </form>
  <p class="muted small">
    Stored in this browser only. Every API call sends it as
    <span class="mono">Authorization: Bearer</span>; it never enters the URL or
    the build.
  </p>

  {#if view.version}
    <table>
      <tbody>
        <tr><th>Product</th><td class="mono">{view.version.product}</td></tr>
        <tr><th>API</th><td class="mono">{view.version.api}</td></tr>
        <tr><th>Image digest</th><td class="mono">{view.version.image_digest ?? "unknown"}</td></tr>
      </tbody>
    </table>
  {/if}

  {#if view.health}
    <table data-testid="health-report">
      <thead>
        <tr><th>Service</th><th>State</th><th>Detail</th></tr>
      </thead>
      <tbody>
        {#each view.health.services as service (service.name)}
          <tr>
            <td class="mono">{service.name}</td>
            <td>{service.state}</td>
            <td class="muted">{service.detail ?? ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="muted small">
      Queue depth {view.health.queue_depth}, workers {view.health.workers}.
    </p>
  {/if}
</section>

<style>
  section { max-width: 60rem; }

  form { flex-wrap: wrap; }

  input {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.4rem 0.6rem;
    min-width: 18rem;
  }
</style>
