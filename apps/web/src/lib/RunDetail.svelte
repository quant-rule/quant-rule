<script lang="ts">
  import { errorText } from "./api";
  import { downloadArtifact, saveArtifact } from "./artifacts";
  import { verifyRun } from "./inspect";
  import { fetchRunLineage } from "./lineage";
  import { bytes, elapsed, isActive, shortId } from "./runs";
  import type {
    ArtifactRef,
    RunDetail,
    RunDigest,
    RunLineage,
    RunState,
    StageStatus,
  } from "./types";

  let {
    detail,
    loading,
    onBack,
    onCancel,
  }: {
    detail: RunDetail | null;
    loading: boolean;
    onBack: () => void;
    onCancel: (runId: string) => void;
  } = $props();

  let copied = $state(false);
  let copyError = $state<string | null>(null);
  let verifying = $state(false);
  let verified = $state<RunDigest | null>(null);
  let verifyError = $state<string | null>(null);
  let downloading = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let saved = $state<string | null>(null);
  let lineage = $state<RunLineage | null>(null);
  let lineageLoaded = $state(false);
  let lineageError = $state<string | null>(null);
  let lineageRunId: string | null = null;

  // A run record is the run; its lineage is a separate read. A recorded absence
  // is an empty state, so it must not turn the run into an error.
  $effect(() => {
    const runId = detail?.run_id ?? null;
    if (runId === null) {
      lineageRunId = null;
      lineage = null;
      lineageLoaded = false;
      lineageError = null;
      return;
    }
    if (lineageRunId === runId) {
      return;
    }
    lineageRunId = runId;
    lineage = null;
    lineageLoaded = false;
    lineageError = null;
    void (async () => {
      try {
        const value = await fetchRunLineage(runId);
        if (lineageRunId === runId) {
          lineage = value;
          lineageLoaded = true;
        }
      } catch (error) {
        if (lineageRunId === runId) {
          lineageError = errorText(error);
          lineageLoaded = true;
        }
      }
    })();
  });

  async function copy(): Promise<void> {
    if (!detail?.digest) {
      return;
    }
    if (!navigator.clipboard) {
      copyError = "Copying requires HTTPS or localhost; this page is not a secure context";
      return;
    }
    try {
      await navigator.clipboard.writeText(detail.digest);
      copied = true;
      copyError = null;
    } catch (error) {
      copyError = errorText(error);
    }
  }

  /** Read the recorded digest; this is the `verify` command's use case. */
  async function verify(): Promise<void> {
    if (!detail) {
      return;
    }
    verifying = true;
    verifyError = null;
    try {
      verified = await verifyRun(detail.run_id);
    } catch (error) {
      verified = null;
      verifyError = errorText(error);
    } finally {
      verifying = false;
    }
  }

  /** Fetch one artifact and offer it only when the bytes match the record. */
  async function download(artifact: ArtifactRef): Promise<void> {
    if (!detail) {
      return;
    }
    downloading = artifact.name;
    downloadError = null;
    saved = null;
    try {
      const body = await downloadArtifact(detail.run_id, artifact);
      saveArtifact(body, artifact.name);
      saved = artifact.name;
    } catch (error) {
      downloadError = errorText(error);
    } finally {
      downloading = null;
    }
  }

  function statusBadge(status: RunState): string {
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

  function stageBadge(status: StageStatus): string {
    switch (status) {
      case "done":
        return "ok";
      case "failed":
        return "error";
      case "running":
        return "warn";
      case "pending":
      case "skipped":
        return "";
    }
  }

</script>

<section class="stack" data-testid="run-status">
  <div class="row">
    <button onclick={onBack} aria-label="Back to runs">Back</button>
    <h2>Run detail</h2>
    {#if detail}
      <span class="mono" title={detail.run_id}>{shortId(detail.run_id)}</span>
      <span class="badge {statusBadge(detail.status)}">{detail.status}</span>
      {#if isActive(detail.status)}
        <button onclick={() => onCancel(detail.run_id)}>Cancel</button>
      {/if}
    {/if}
  </div>

  {#if loading && !detail}
    <div class="notice">Loading run…</div>
  {:else if !detail}
    <div class="notice">The run is unavailable.</div>
  {:else}
    <p class="muted small">Owner {detail.owner}</p>

    {#if detail.error}
      <div class="notice error" role="alert">{detail.error}</div>
    {/if}

    <h3>Stages</h3>
    <ol class="stages">
      {#each detail.stages as stage (stage.name)}
        <li>
          <span class="mono">{stage.name}</span>
          <span class="badge {stageBadge(stage.state)}">{stage.state}</span>
          <span class="muted small">{elapsed(stage.elapsed_ms)}</span>
        </li>
      {/each}
    </ol>

    <h3>Provenance</h3>
    <table>
      <tbody>
        <tr>
          <th>Data version</th>
          <td class="mono" title={detail.provenance.data_version}>
            {shortId(detail.provenance.data_version)}
          </td>
        </tr>
        <tr>
          <th>Config hash</th>
          <td class="mono" title={detail.provenance.config_hash}>
            {shortId(detail.provenance.config_hash)}
          </td>
        </tr>
        <tr>
          <th>Model identity</th>
          <td class="mono">{detail.provenance.model_version}</td>
        </tr>
        <tr>
          <th>Frozen model artifact</th>
          <td class="mono" title={detail.model_artifact_id ?? "none"}>
            {detail.model_artifact_id == null
              ? "none"
              : shortId(detail.model_artifact_id)}
            {#if detail.model_artifact_id != null}
              <span class="badge warn">diagnostic input</span>
            {/if}
          </td>
        </tr>
        <tr>
          <th>Image digest</th>
          <td><pre class="mono digest">{detail.provenance.image_digest}</pre></td>
        </tr>
      </tbody>
    </table>

    <div class="stack" data-testid="run-lineage">
      <div class="row">
        <h3>Lineage</h3>
        <span class="badge">recorded references · not evidence</span>
      </div>
      {#if lineageError}
        <div class="notice error" role="alert">{lineageError}</div>
      {:else if !lineageLoaded}
        <div class="notice small">Loading lineage…</div>
      {:else if lineage === null}
        <div class="notice">No recorded lineage</div>
      {:else}
        <p class="muted small">
          Recorded {lineage.recorded_at}. The set is immutable; a later pack retire
          does not rewrite what this run cited.
        </p>
        <table>
          <thead>
            <tr><th>Kind</th><th>Id</th><th>Hash</th><th>Pack</th></tr>
          </thead>
          <tbody>
            {#each lineage.references as reference}
              <tr>
                <td>{reference.kind}</td>
                <td class="mono" title={reference.id}>{shortId(reference.id)}</td>
                <td class="mono" title={reference.hash}>{shortId(reference.hash)}</td>
                <td>
                  {#if reference.pack}
                    {reference.pack.name} {reference.pack.version}
                    <span class="badge">{reference.pack.state}</span>
                  {:else}
                    none
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    {#if detail.digest}
      <div class="row">
        <h3>Canonical digest</h3>
        <button data-testid="run-verify" onclick={() => void verify()} disabled={verifying} aria-label="Verify canonical digest">
          {verifying ? "Verifying…" : "Verify"}
        </button>
        <button onclick={() => void copy()} aria-label="Copy canonical digest">
          {copied ? "Copied" : "Copy"}
        </button>
        {#if copyError}
          <span class="notice error small" role="alert">{copyError}</span>
        {/if}
      </div>
      <pre class="mono digest">{detail.digest}</pre>
      {#if verified}
        <div class="notice small">
          The run record reports <span class="mono">{verified.digest}</span>
          {#if verified.digest === detail.digest}
            and matches the digest above.
          {:else}
            and differs from the digest above.
          {/if}
        </div>
      {/if}
      {#if verifyError}
        <div class="notice error" role="alert">{verifyError}</div>
      {/if}
    {/if}

    {#if detail.status !== "failed"}
      <h3>Artifacts</h3>
      {#if detail.artifacts.length === 0}
        <div class="notice">No artifacts yet.</div>
      {:else}
        <table>
          <thead>
            <tr><th>Name</th><th>Size</th><th>SHA-256</th><th>Download</th></tr>
          </thead>
          <tbody>
            {#each detail.artifacts as artifact (artifact.name)}
              <tr>
                <td class="mono">{artifact.name}</td>
                <td>{bytes(artifact.bytes)}</td>
                <td class="mono" title={artifact.sha256}>{shortId(artifact.sha256)}</td>
                <td>
                  <button
                    data-testid="run-fetch"
                    onclick={() => void download(artifact)}
                    disabled={downloading !== null}
                    aria-label={`Download ${artifact.name}`}
                  >
                    {downloading === artifact.name ? "Checking…" : "Download"}
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if saved}
          <div class="notice small">{saved} matched the record and was saved.</div>
        {/if}
        {#if downloadError}
          <div class="notice error" role="alert">{downloadError}</div>
        {/if}
      {/if}
    {/if}
  {/if}
</section>

<style>
  section {
    max-width: 80rem;
    gap: 0.8rem;
  }

  .stages {
    display: grid;
    gap: 0.35rem;
    margin: 0;
    padding-left: 1.2rem;
  }

  .stages li {
    display: flex;
    gap: 0.6rem;
    align-items: center;
  }

  .digest {
    margin: 0;
    padding: 0.5rem 0.6rem;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    font-size: 0.82rem;
  }
</style>
