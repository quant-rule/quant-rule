<script lang="ts">
  import { isTauri } from "./api";
  import { pickDirectory, pickSaveFile } from "./paths";

  let {
    workspaceRoot = $bindable(),
    memoryPath = $bindable(),
  }: {
    workspaceRoot: string;
    memoryPath: string;
  } = $props();

  const native = isTauri();

  async function browseWorkspace() {
    const selected = await pickDirectory("Choose a workspace root");
    if (selected) {
      workspaceRoot = selected;
    }
  }

  async function browseMemory() {
    const fallback = workspaceRoot
      ? `${workspaceRoot.replace(/[\\/]$/, "")}/research.db`
      : "";
    const selected = await pickSaveFile(
      "Choose the research memory database",
      memoryPath || fallback,
      [{ name: "SQLite database", extensions: ["db", "sqlite"] }],
    );
    if (selected) {
      memoryPath = selected;
    }
  }
</script>

<div class="stack">
  <div class="card">
    <h2>Local paths</h2>
    <p class="muted">
      QuantRule keeps candles, configurations, run summaries, and the research
      memory on this machine. There is no account and no network request.
    </p>

    <label class="field">
      <span>Workspace root</span>
      <div class="row">
        <input bind:value={workspaceRoot} placeholder="/home/you/quant-rule-workspace" />
        {#if native}
          <button type="button" onclick={browseWorkspace}>Browse…</button>
        {/if}
      </div>
      <small class="muted">
        A directory with <code>config/</code> and <code>data/sources/</code>.
      </small>
    </label>

    <label class="field">
      <span>Research memory database</span>
      <div class="row">
        <input bind:value={memoryPath} placeholder="/home/you/quant-rule-workspace/research.db" />
        {#if native}
          <button type="button" onclick={browseMemory}>Browse…</button>
        {/if}
      </div>
      <small class="muted">
        A local SQLite file. The app creates it when you open the memory.
      </small>
    </label>
  </div>

  <div class="card">
    <h2>Boundary</h2>
    <ul class="muted">
      <li>No login, subscription, or cloud compute.</li>
      <li>No engine reads the clock or a network resource.</li>
      <li>The UI calls only the shared application service.</li>
      <li>The pipeline stages are being moved into the shared service; until then they remain on the CLI.</li>
    </ul>
  </div>
</div>

<style>
  .field {
    display: grid;
    gap: 0.35rem;
    margin-top: 0.9rem;
  }

  .field > span {
    font-weight: 600;
  }

  .field input {
    width: 100%;
  }
</style>
