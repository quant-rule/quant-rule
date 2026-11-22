<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { saveArtifact } from "./artifacts";
  import { downloadPack, type PackRecord, type PackState } from "./packs";
  import {
    PACK_KINDS,
    PACK_STATES,
    create as registerPack,
    load as loadPacks,
    packs,
    select as selectPack,
    transition as transitionPack,
  } from "./packs.svelte";
  import { bytes, shortId, submitted } from "./runs";
  import { downloadSignal } from "./signals";
  import {
    create as runSignalCreate,
    load as loadSignals,
    select as selectSignal,
    signals,
  } from "./signals.svelte";

  type Tab = "packs" | "signals";

  let tab = $state<Tab>("packs");
  let kind = $state<"" | (typeof PACK_KINDS)[number]>("");
  let packState = $state<"" | PackState>("");
  let modelArtifactId = $state("");
  let dataVersion = $state("");
  let configId = $state("");
  let strategyPackId = $state("");
  let downloading = $state<string | null>(null);
  let saved = $state<string | null>(null);
  let downloadError = $state<string | null>(null);

  const packView = $derived(packs());
  const signalView = $derived(signals());
  const frozenPacks = $derived(
    packView.items.filter(
      (item) => item.kind === "strategy" && item.state === "frozen",
    ),
  );
  const selectedPack = $derived(
    packView.detail?.summary.pack_id === packView.selected
      ? packView.detail
      : null,
  );
  const selectedSignal = $derived(
    signalView.detail?.summary.artifact_id === signalView.selected
      ? signalView.detail
      : null,
  );
  const canCreateSignal = $derived(
    !signalView.unavailable &&
      !signalView.creating &&
      modelArtifactId.trim() !== "" &&
      dataVersion.trim() !== "" &&
      configId.trim() !== "" &&
      strategyPackId !== "",
  );

  onMount(() => {
    void loadPacks();
    void loadSignals();
  });

  async function filterPacks(): Promise<void> {
    await loadPacks({
      ...(kind === "" ? {} : { kind }),
      ...(packState === "" ? {} : { state: packState }),
    });
  }

  async function uploadPack(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (file === undefined) {
      return;
    }
    await registerPack(file);
    input.value = "";
  }

  async function downloadSelectedPack(record: PackRecord): Promise<void> {
    downloading = record.summary.pack_id;
    downloadError = null;
    saved = null;
    try {
      const body = await downloadPack(record);
      const name = `${shortId(record.summary.pack_id)}.json`;
      saveArtifact(body, name);
      saved = name;
    } catch (error) {
      downloadError = errorText(error);
    } finally {
      downloading = null;
    }
  }

  async function downloadSelectedSignal(): Promise<void> {
    const record = selectedSignal;
    if (record === null) {
      return;
    }
    downloading = record.summary.artifact_id;
    downloadError = null;
    saved = null;
    try {
      const body = await downloadSignal(record);
      const name = `${shortId(record.summary.artifact_id)}.json`;
      saveArtifact(body, name);
      saved = name;
    } catch (error) {
      downloadError = errorText(error);
    } finally {
      downloading = null;
    }
  }

  async function submitSignal(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    await runSignalCreate({
      model_artifact_id: modelArtifactId.trim(),
      data_version: dataVersion.trim(),
      config_id: configId.trim(),
      strategy_pack_id: strategyPackId,
    });
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Research artifacts</h2>
    <span class="badge">packs are definitions · not evidence</span>
    <span class="badge">signals are computed alpha · not evidence</span>
  </div>

  <div class="row" role="tablist" aria-label="Research artifact tabs">
    <button
      role="tab"
      aria-selected={tab === "packs"}
      class:active={tab === "packs"}
      onclick={() => (tab = "packs")}
    >
      Packs
    </button>
    <button
      role="tab"
      aria-selected={tab === "signals"}
      class:active={tab === "signals"}
      onclick={() => (tab = "signals")}
    >
      Signals
    </button>
  </div>

  {#if tab === "packs"}
    <p class="muted small">
      A pack is an immutable definition. Freeze is a human decision backed by a
      Quant Validator record; a change is a new pack.
    </p>

    <div class="row">
      <label>
        Kind
        <select bind:value={kind} onchange={() => void filterPacks()}>
          <option value="">all</option>
          {#each PACK_KINDS as option (option)}<option value={option}>{option}</option>{/each}
        </select>
      </label>
      <label>
        State
        <select bind:value={packState} onchange={() => void filterPacks()}>
          <option value="">all</option>
          {#each PACK_STATES as option (option)}<option value={option}>{option}</option>{/each}
        </select>
      </label>
      <label class="create">
        <span>Register pack</span>
        <input
          type="file"
          accept="application/json,.json"
          data-testid="pack-create"
          onchange={(event) => void uploadPack(event)}
          disabled={packView.acting !== null}
        />
      </label>
      <button onclick={() => void loadPacks()} disabled={packView.loading}>Refresh</button>
    </div>

    {#if packView.error}
      <div class="notice error" role="alert">{packView.error}</div>
    {/if}
    {#if packView.created}
      <div class="notice small" data-testid="pack-created">
        Registered {shortId(packView.created.summary.pack_id)} in state
        {packView.created.summary.state}.
      </div>
    {/if}

    {#if packView.loading && packView.items.length === 0}
      <div class="notice">Loading packs…</div>
    {:else if packView.items.length === 0}
      <div class="notice">No packs. Register one above.</div>
    {:else}
      <table data-testid="packs-table" aria-busy={packView.loading}>
        <thead>
          <tr>
            <th>Pack</th><th>Kind</th><th>State</th><th>Name</th><th>Version</th>
            <th>Owner</th><th>Bytes</th><th>Created (UTC)</th>
          </tr>
        </thead>
        <tbody>
          {#each packView.items as item (item.pack_id)}
            <tr>
              <td>
                <button class="link" onclick={() => void selectPack(item.pack_id)} aria-label={`Open pack ${item.pack_id}`}>
                  {shortId(item.pack_id)}
                </button>
              </td>
              <td>{item.kind}</td>
              <td><span class="badge">{item.state}</span></td>
              <td>{item.name}</td>
              <td>{item.version}</td>
              <td>{item.owner}</td>
              <td>{bytes(item.bytes)}</td>
              <td class="mono">{submitted(item.created_at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    {#if packView.detailLoading}
      <div class="notice">Loading pack…</div>
    {:else if selectedPack}
      <div class="stack" data-testid="pack-detail">
        <div class="row">
          <h3>Pack detail</h3>
          <span class="badge">definition · not evidence</span>
        </div>
        <table>
          <tbody>
            <tr><th>Pack id</th><td class="mono">{selectedPack.summary.pack_id}</td></tr>
            <tr><th>Kind</th><td>{selectedPack.summary.kind}</td></tr>
            <tr><th>State</th><td>{selectedPack.summary.state}</td></tr>
            <tr><th>Name</th><td>{selectedPack.summary.name}</td></tr>
            <tr><th>Version</th><td>{selectedPack.summary.version}</td></tr>
            <tr><th>Owner</th><td>{selectedPack.summary.owner}</td></tr>
            <tr><th>Object key</th><td class="mono">{selectedPack.object_key}</td></tr>
            <tr><th>SHA-256</th><td class="mono">{selectedPack.sha256}</td></tr>
            <tr><th>Validations</th><td>{selectedPack.validations.length}</td></tr>
            <tr><th>Events</th><td>{selectedPack.events.length}</td></tr>
          </tbody>
        </table>
        <div class="row">
          <button
            data-testid="pack-validate"
            onclick={() => void transitionPack(selectedPack.summary.pack_id, "validate")}
            disabled={packView.acting !== null || selectedPack.summary.state !== "draft"}
          >Validate</button>
          <button
            data-testid="pack-freeze"
            onclick={() => void transitionPack(selectedPack.summary.pack_id, "freeze")}
            disabled={packView.acting !== null || selectedPack.summary.state !== "validated"}
          >Freeze</button>
          <button
            data-testid="pack-retire"
            onclick={() => void transitionPack(selectedPack.summary.pack_id, "retire")}
            disabled={packView.acting !== null || selectedPack.summary.state !== "frozen"}
          >Retire</button>
          <button
            data-testid="pack-fetch"
            onclick={() => void downloadSelectedPack(selectedPack)}
            disabled={downloading !== null}
          >Download</button>
        </div>
        {#if saved}
          <div class="notice small">{saved} matched the record and was saved.</div>
        {/if}
        {#if downloadError}
          <div class="notice error" role="alert">{downloadError}</div>
        {/if}
      </div>
    {/if}
  {:else}
    <p class="muted small">
      A signal is computed alpha for one frozen model artifact and one frozen
      signal-mode strategy pack. The list is metadata only; the download is the
      document.
    </p>

    <form class="stack" onsubmit={(event) => void submitSignal(event)}>
      <div class="row">
        <label>Model artifact id <input bind:value={modelArtifactId} placeholder="sha256:…" /></label>
        <label>Data version <input bind:value={dataVersion} placeholder="sha256:…" /></label>
        <label>Config id <input bind:value={configId} placeholder="cfg_…" /></label>
        <label>
          Strategy pack
          <select bind:value={strategyPackId}>
            <option value="">select a frozen strategy pack</option>
            {#each frozenPacks as item (item.pack_id)}
              <option value={item.pack_id}>{item.name} {item.version}</option>
            {/each}
          </select>
        </label>
        <button
          type="submit"
          data-testid="signal-create"
          disabled={!canCreateSignal}
        >{signalView.creating ? "Computing…" : "Create signal"}</button>
      </div>
      {#if frozenPacks.length === 0}
        <div class="notice small">Freeze a strategy pack before creating a signal.</div>
      {/if}
    </form>

    {#if signalView.error}
      <div class="notice error" role="status">{signalView.error}</div>
    {/if}
    {#if signalView.created}
      <div class="notice small" data-testid="signal-created">
        Created {shortId(signalView.created.artifact.summary.artifact_id)} with
        {bytes(signalView.created.artifact.summary.bytes)}.
      </div>
    {/if}

    {#if signalView.loading && signalView.items.length === 0}
      <div class="notice">Loading signals…</div>
    {:else if signalView.items.length === 0}
      <div class="notice">No signals. Create one above.</div>
    {:else}
      <table data-testid="signal-table" aria-busy={signalView.loading}>
        <thead>
          <tr><th>Signal</th><th>Kind</th><th>Bytes</th><th>Created (UTC)</th></tr>
        </thead>
        <tbody>
          {#each signalView.items as item (item.summary.artifact_id)}
            <tr>
              <td>
                <button class="link" onclick={() => void selectSignal(item.summary.artifact_id)} aria-label={`Open signal ${item.summary.artifact_id}`}>
                  {shortId(item.summary.artifact_id)}
                </button>
              </td>
              <td>{item.summary.kind}</td>
              <td>{bytes(item.summary.bytes)}</td>
              <td class="mono">{submitted(item.summary.created_at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    {#if signalView.detailLoading}
      <div class="notice">Loading signal…</div>
    {:else if selectedSignal}
      <div class="stack" data-testid="signal-detail">
        <div class="row">
          <h3>Signal detail</h3>
          <span class="badge">computed alpha · not evidence</span>
        </div>
        <table>
          <tbody>
            <tr><th>Signal id</th><td class="mono">{selectedSignal.summary.artifact_id}</td></tr>
            <tr><th>Kind</th><td>{selectedSignal.summary.kind}</td></tr>
            <tr><th>Bytes</th><td>{bytes(selectedSignal.summary.bytes)}</td></tr>
            <tr><th>Created (UTC)</th><td class="mono">{selectedSignal.summary.created_at}</td></tr>
            <tr><th>Object key</th><td class="mono">{selectedSignal.object_key}</td></tr>
            <tr><th>SHA-256</th><td class="mono">{selectedSignal.sha256}</td></tr>
          </tbody>
        </table>
        <div class="row">
          <button
            data-testid="signal-fetch"
            onclick={() => void downloadSelectedSignal()}
            disabled={downloading !== null}
          >Download</button>
        </div>
        {#if saved}
          <div class="notice small">{saved} matched the record and was saved.</div>
        {/if}
        {#if downloadError}
          <div class="notice error" role="alert">{downloadError}</div>
        {/if}
      </div>
    {/if}
  {/if}
</section>

<style>
  section {
    max-width: 80rem;
  }

  .create {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  form {
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 0.8rem;
    gap: 0.6rem;
  }

  .link {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--accent);
    font-family: var(--mono);
    cursor: pointer;
  }

  button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
