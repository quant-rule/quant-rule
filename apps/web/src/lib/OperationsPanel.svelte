<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { connection } from "./connection.svelte";
  import { verifyRun } from "./inspect";
  import {
    cleanupCounts,
    cleanupTime,
    retentionWindow,
  } from "./operations";
  import { load, operations, reset } from "./operations.svelte";
  import { profiles } from "./profiles.svelte";
  import type { RunDigest } from "./types";

  const link = $derived(connection());
  const profile = $derived(profiles());
  const view = $derived(operations());

  let runId = $state("");
  let verifying = $state(false);
  let verified = $state<RunDigest | null>(null);
  let verifyError = $state<string | null>(null);

  const label = $derived(
    link.state === "connected"
      ? "connected"
      : link.state === "degraded"
        ? "degraded"
        : link.state === "mismatch"
          ? "version mismatch"
          : link.state === "unreachable"
            ? "unreachable"
            : link.state === "unauthorized"
              ? "token required"
              : "unknown",
  );

  onMount(() => {
    void load();
    return () => reset();
  });

  async function verify(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const id = runId.trim();
    if (id === "" || verifying) {
      return;
    }
    verifying = true;
    verified = null;
    verifyError = null;
    try {
      verified = await verifyRun(id);
    } catch (error) {
      verifyError = errorText(error);
    } finally {
      verifying = false;
    }
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Operations</h2>
    <button onclick={() => void load()} disabled={view.loading}>
      {view.loading ? "Reading…" : "Refresh"}
    </button>
  </div>

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  <section data-testid="operations-health">
    <h3>Gateway and image</h3>
    <table>
      <tbody>
        <tr><th>Connection</th><td>{label}</td></tr>
        <tr><th>Product</th><td class="mono">{link.version?.product ?? "unknown"}</td></tr>
        <tr><th>API</th><td class="mono">{link.version?.api ?? "unknown"}</td></tr>
        <tr>
          <th>Image digest</th>
          <td class="mono">{link.version?.image_digest ?? "unknown"}</td>
        </tr>
        <tr>
          <th>Health</th>
          <td>{link.health?.state ?? "unknown"}</td>
        </tr>
      </tbody>
    </table>
    <p class="muted small">
      The image digest identifies the build that executed a run. A different
      digest is a different execution.
    </p>
  </section>

  <section data-testid="operations-profile">
    <h3>Active profile</h3>
    <table>
      <tbody>
        <tr><th>Profile</th><td class="mono">{profile.active}</td></tr>
        <tr>
          <th>Bearer token</th>
          <td>{link.tokenStored ? "Stored in this browser" : "Not stored"}</td>
        </tr>
      </tbody>
    </table>
    <p class="muted small">
      Every retention and verification read carries the active profile. The
      token is never rendered; Connection owns entering or forgetting it.
    </p>
  </section>

  <section data-testid="operations-retention">
    <h3>Diagnostic stage retention</h3>
    {#if view.report}
      <table>
        <tbody>
          <tr>
            <th>Retention window</th>
            <td>{retentionWindow(view.report.stage_retention_days)}</td>
          </tr>
          <tr>
            <th>Last startup cleanup</th>
            <td class="mono">{cleanupTime(view.report)}</td>
          </tr>
          <tr>
            <th>Removed</th>
            <td>{cleanupCounts(view.report.last_cleanup)}</td>
          </tr>
        </tbody>
      </table>
      {#if !view.report.stage_store}
        <p class="muted small">
          No stage store is configured, so no startup cleanup runs.
        </p>
      {/if}
    {:else if view.loading}
      <p class="muted">Reading retention status…</p>
    {:else}
      <p class="muted">Retention status is unavailable.</p>
    {/if}
    <p>
      Only diagnostic stage artifacts expire. Runs, research records, and
      evidence are permanent.
    </p>
  </section>

  <section class="stack">
    <h3>Run verification</h3>
    <form
      class="row"
      data-testid="operations-verify"
      onsubmit={(event) => void verify(event)}
    >
      <label>
        Run id
        <input
          bind:value={runId}
          placeholder="sha256:…"
          aria-label="Run id to verify"
          spellcheck="false"
        />
      </label>
      <button type="submit" disabled={runId.trim() === "" || verifying}>
        {verifying ? "Verifying…" : "Verify digest"}
      </button>
    </form>
    {#if verified}
      <p>
        Recorded canonical digest:
        <span class="mono" data-testid="operations-verify-digest">{verified.digest}</span>
      </p>
    {/if}
    {#if verifyError}
      <div class="notice error" role="alert">{verifyError}</div>
    {/if}
    <p class="muted small">
      This reads the canonical digest the run record already carries. Compare
      the digests of two equivalent runs, as <code>make verify</code> and the
      composed tier do, to prove the deployment reproduces. It is not research
      evidence, a baseline, or a claim about profitability.
    </p>
  </section>
</section>

<style>
  section { max-width: 64rem; }
  form { flex-wrap: wrap; }
  input {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.4rem 0.6rem;
    min-width: 24rem;
  }
</style>
