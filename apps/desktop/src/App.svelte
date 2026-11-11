<script lang="ts">
  import { call } from "./lib/api";
  import { errorMessage } from "./lib/errors";
  import type { WorkspaceView } from "./lib/types";
  import ConfigPanel from "./lib/ConfigPanel.svelte";
  import DataPanel from "./lib/DataPanel.svelte";
  import MemoryPanel from "./lib/MemoryPanel.svelte";
  import RunPanel from "./lib/RunPanel.svelte";
  import SettingsPanel from "./lib/SettingsPanel.svelte";
  import WorkspacePanel from "./lib/WorkspacePanel.svelte";

  type Section = "workspace" | "data" | "configs" | "run" | "memory" | "settings";

  const sections: { id: Section; label: string; hint: string }[] = [
    { id: "workspace", label: "Workspace", hint: "Open a local root" },
    { id: "data", label: "Data", hint: "Inspect candle sources" },
    { id: "configs", label: "Configs", hint: "Validate a run" },
    { id: "run", label: "Run", hint: "Model and evidence" },
    { id: "memory", label: "Memory", hint: "Local research store" },
    { id: "settings", label: "Settings", hint: "Paths and defaults" },
  ];

  let section = $state<Section>("workspace");
  let workspaceRoot = $state(localStorage.getItem("quantrule.workspace") ?? "");
  let memoryPath = $state(localStorage.getItem("quantrule.memory") ?? "");
  let workspace = $state<WorkspaceView | null>(null);
  let selectedRun = $state("");
  let busy = $state(false);
  let status = $state("");
  let error = $state("");

  $effect(() => {
    localStorage.setItem("quantrule.workspace", workspaceRoot);
  });
  $effect(() => {
    localStorage.setItem("quantrule.memory", memoryPath);
  });

  async function openWorkspace() {
    busy = true;
    error = "";
    status = "";
    try {
      workspace = await call<WorkspaceView>("open_workspace", { root: workspaceRoot });
      selectedRun = workspace.runs[0] ?? "";
      status = `Opened ${workspace.datasets.length} dataset(s) and ${workspace.runs.length} run config(s).`;
      section = "workspace";
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  function selectRun(path: string) {
    selectedRun = path;
    section = "run";
  }

  function navigate(target: Section) {
    section = target;
  }
</script>

<div class="app">
  <aside class="sidebar">
    <div class="brand">
      <div class="mark">Q</div>
      <div>
        <strong>QuantRule</strong>
        <div class="muted small">Local research workstation</div>
      </div>
    </div>

    <nav aria-label="Sections">
      {#each sections as item (item.id)}
        <button
          class:active={section === item.id}
          aria-current={section === item.id ? "page" : undefined}
          onclick={() => navigate(item.id)}
        >
          <span>{item.label}</span>
          <small>{item.hint}</small>
        </button>
      {/each}
    </nav>

    <div class="foot">
      <div class="muted small">No account. No cloud. Local only.</div>
    </div>
  </aside>

  <main>
    <header class="topbar">
      <div>
        <h1>{sections.find((item) => item.id === section)?.label}</h1>
        <div class="muted small mono">
          {workspace ? workspace.root : workspaceRoot || "No workspace open"}
        </div>
      </div>
      <div class="row">
        <span class="badge">{workspace ? "workspace open" : "workspace closed"}</span>
      </div>
    </header>

    {#if status}
      <div class="notice ok">{status}</div>
    {/if}
    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <section class="content">
      {#if section === "workspace"}
        <WorkspacePanel
          root={workspaceRoot}
          {workspace}
          {busy}
          onOpen={openWorkspace}
        />
      {:else if section === "data"}
        <DataPanel datasets={workspace?.datasets ?? []} />
      {:else if section === "configs"}
        <ConfigPanel runs={workspace?.runs ?? []} onSelect={selectRun} />
      {:else if section === "run"}
        <RunPanel runPath={selectedRun} />
      {:else if section === "memory"}
        <MemoryPanel {memoryPath} runPath={selectedRun} />
      {:else if section === "settings"}
        <SettingsPanel bind:workspaceRoot bind:memoryPath />
      {/if}
    </section>
  </main>
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: 250px minmax(0, 1fr);
    min-height: 100%;
  }

  .sidebar {
    background: #0b0e13;
    border-right: 1px solid var(--line);
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .brand {
    display: flex;
    gap: 0.65rem;
    align-items: center;
    padding: 0.35rem 0.2rem 0.8rem;
    border-bottom: 1px solid var(--line);
  }

  .mark {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    color: #06211d;
    display: grid;
    place-items: center;
    font-weight: 800;
  }

  nav {
    display: grid;
    gap: 0.3rem;
  }

  nav button {
    display: grid;
    gap: 0.1rem;
    background: transparent;
    border-color: transparent;
    text-align: left;
    padding: 0.6rem 0.7rem;
  }

  nav button small {
    color: var(--muted);
  }

  nav button.active {
    background: var(--panel);
    border-color: var(--line);
  }

  nav button.active span {
    color: var(--accent);
  }

  .foot {
    margin-top: auto;
    border-top: 1px solid var(--line);
    padding-top: 0.8rem;
  }

  main {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--line);
    background: #11161e;
  }

  .topbar h1 {
    margin-bottom: 0.15rem;
  }

  .badge {
    border: 1px solid var(--line);
    border-radius: 999px;
    padding: 0.25rem 0.65rem;
    color: var(--muted);
    font-size: 0.78rem;
  }

  .notice {
    margin: 0.8rem 1.25rem 0;
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    font-size: 0.88rem;
  }

  .content {
    padding: 1.25rem;
    min-width: 0;
  }

  .small {
    font-size: 0.8rem;
  }

  @media (max-width: 800px) {
    .app {
      grid-template-columns: 1fr;
    }

    .sidebar {
      border-right: none;
      border-bottom: 1px solid var(--line);
    }

    nav {
      grid-template-columns: repeat(2, 1fr);
    }

    .foot {
      display: none;
    }
  }
</style>
