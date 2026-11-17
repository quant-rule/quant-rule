<script lang="ts">
  import { onMount } from "svelte";
  import ConfigsPanel from "./lib/ConfigsPanel.svelte";
  import ConnectionPanel from "./lib/ConnectionPanel.svelte";
  import DatasetsPanel from "./lib/DatasetsPanel.svelte";
  import HelpPanel from "./lib/HelpPanel.svelte";
  import InspectPanel from "./lib/InspectPanel.svelte";
  import ProfilesPanel from "./lib/ProfilesPanel.svelte";
  import ResearchPanel from "./lib/ResearchPanel.svelte";
  import RunsPanel from "./lib/RunsPanel.svelte";
  import ProfileSwitcher from "./lib/ProfileSwitcher.svelte";
  import SimulationChartHarness from "./lib/SimulationChartHarness.svelte";
  import SimulationRunsPanel from "./lib/SimulationRunsPanel.svelte";
  import WorkbenchPanel from "./lib/WorkbenchPanel.svelte";
  import { SECTIONS, type Section } from "./lib/capabilities";
  import { initialise } from "./lib/connection.svelte";
  import { load as loadProfiles } from "./lib/profiles.svelte";

  const sections = SECTIONS;

  let section = $state<Section>("connection");
  const fixture = new URLSearchParams(window.location.search).get("fixture");

  onMount(() => {
    if (fixture === "simulation-chart") {
      return;
    }
    // Apply the stored token first, then load the profile list with it.
    void (async () => {
      await initialise();
      await loadProfiles();
    })();
  });
</script>

{#if fixture === "simulation-chart"}
  <SimulationChartHarness />
{:else}
<div class="app">
  <aside class="sidebar">
    <div class="brand">
      <div class="mark">Q</div>
      <div>
        <strong>Quant Rule</strong>
        <div class="muted small">Trading edge research</div>
      </div>
    </div>

    <nav aria-label="Sections">
      {#each sections as item (item.id)}
        <button
          class:active={section === item.id}
          aria-current={section === item.id ? "page" : undefined}
          onclick={() => (section = item.id)}
        >
          <span>{item.label}</span>
          <small>{item.hint}</small>
        </button>
      {/each}
    </nav>

    <div class="foot">
      <div class="muted small">No account. One researcher.</div>
    </div>
  </aside>

  <main>
    <header class="topbar">
      <h1>{sections.find((item) => item.id === section)?.label}</h1>
      <ProfileSwitcher />
    </header>

    <section class="content">
      {#if section === "connection"}
        <ConnectionPanel />
      {:else if section === "profiles"}
        <ProfilesPanel />
      {:else if section === "workbench"}
        <WorkbenchPanel />
      {:else if section === "datasets"}
        <DatasetsPanel />
      {:else if section === "configs"}
        <ConfigsPanel onOpenRuns={() => (section = "runs")} />
      {:else if section === "runs"}
        <RunsPanel />
      {:else if section === "research"}
        <ResearchPanel />
      {:else if section === "simulation"}
        <SimulationRunsPanel />
      {:else if section === "inspect"}
        <InspectPanel />
      {:else if section === "help"}
        <HelpPanel />
      {/if}
    </section>
  </main>
</div>
{/if}

<style>
  .app {
    display: grid;
    grid-template-columns: 250px minmax(0, 1fr);
    min-height: 100vh;
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

  nav { display: grid; gap: 0.3rem; }

  nav button {
    display: grid;
    gap: 0.1rem;
    background: transparent;
    border-color: transparent;
    text-align: left;
    padding: 0.6rem 0.7rem;
  }

  nav button small { color: var(--muted); }
  nav button.active { background: var(--panel); border-color: var(--line); }
  nav button.active span { color: var(--accent); }

  .foot {
    margin-top: auto;
    border-top: 1px solid var(--line);
    padding-top: 0.8rem;
  }

  main { min-width: 0; display: flex; flex-direction: column; }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--line);
    background: #11161e;
  }

  .content { padding: 1.25rem; }

  @media (max-width: 800px) {
    .app { grid-template-columns: 1fr; }
    .sidebar { border-right: none; border-bottom: 1px solid var(--line); }
    nav { grid-template-columns: repeat(2, 1fr); }
    .foot { display: none; }
  }
</style>
