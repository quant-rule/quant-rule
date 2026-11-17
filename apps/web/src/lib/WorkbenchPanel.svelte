<script lang="ts">
  import DatasetsPanel from "./DatasetsPanel.svelte";
  import ModelStagePanel from "./ModelStagePanel.svelte";
  import StrategyStagePanel from "./StrategyStagePanel.svelte";

  let tab = $state<"data" | "model" | "strategy">("data");
</script>

<section class="stack">
  <div class="row" role="tablist" aria-label="Workbench tabs">
    <button
      role="tab"
      aria-selected={tab === "data"}
      class:active={tab === "data"}
      onclick={() => (tab = "data")}
    >
      Data
    </button>
    <button
      role="tab"
      aria-selected={tab === "model"}
      class:active={tab === "model"}
      onclick={() => (tab = "model")}
    >
      Model
    </button>
    <button
      role="tab"
      aria-selected={tab === "strategy"}
      class:active={tab === "strategy"}
      onclick={() => (tab = "strategy")}
    >
      Strategy
    </button>
  </div>

  {#if tab === "data"}
    <DatasetsPanel />
  {:else if tab === "model"}
    <ModelStagePanel />
  {:else}
    <StrategyStagePanel />
  {/if}
</section>

<style>
  section { max-width: 80rem; gap: 0.8rem; }

  button.active {
    background: var(--panel);
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
