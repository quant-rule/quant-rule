<script lang="ts">
  import DatasetsPanel from "./DatasetsPanel.svelte";
  import ModelStagePanel from "./ModelStagePanel.svelte";
  import PortfolioStagePanel from "./PortfolioStagePanel.svelte";
  import StrategyStagePanel from "./StrategyStagePanel.svelte";
  import { clearSelection } from "./stages.svelte";

  let tab = $state<"data" | "model" | "strategy" | "portfolio">("data");

  function switchTab(next: "data" | "model" | "strategy" | "portfolio"): void {
    if (next === tab) {
      return;
    }
    clearSelection();
    tab = next;
  }
</script>

<section class="stack">
  <div class="row" role="tablist" aria-label="Workbench tabs">
    <button
      role="tab"
      aria-selected={tab === "data"}
      class:active={tab === "data"}
      onclick={() => switchTab("data")}
    >
      Data
    </button>
    <button
      role="tab"
      aria-selected={tab === "model"}
      class:active={tab === "model"}
      onclick={() => switchTab("model")}
    >
      Model
    </button>
    <button
      role="tab"
      aria-selected={tab === "strategy"}
      class:active={tab === "strategy"}
      onclick={() => switchTab("strategy")}
    >
      Strategy
    </button>
    <button
      role="tab"
      aria-selected={tab === "portfolio"}
      class:active={tab === "portfolio"}
      onclick={() => switchTab("portfolio")}
    >
      Portfolio
    </button>
  </div>

  {#if tab === "data"}
    <DatasetsPanel />
  {:else if tab === "model"}
    <ModelStagePanel />
  {:else if tab === "strategy"}
    <StrategyStagePanel />
  {:else}
    <PortfolioStagePanel />
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
