<script lang="ts">
  /**
   * The Playwright fixture host for the chart.
   *
   * App renders this only for `?fixture=simulation-chart`. The component still
   * calls the real transport seam; the test mocks the network at the HTTP
   * boundary, so the production path is the one under test.
   */
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import type { ChartMark, ChartSeries } from "./chart/types";
  import SimulationChart from "./SimulationChart.svelte";
  import { fetchSeries } from "./simulation";

  const RUN_ID = "sim_fixture";
  const CHART = 1;

  let series = $state<ChartSeries | null>(null);
  let marks = $state<ChartMark[]>([]);
  let error = $state<string | null>(null);

  onMount(() => {
    void (async () => {
      try {
        series = await fetchSeries(RUN_ID, CHART);
      } catch (failure) {
        error = errorText(failure);
      }
    })();
  });
</script>

<main class="harness">
  <h1>Simulation chart fixture</h1>
  <p class="muted small">
    This page is a fixture host for the browser test; it is not a product screen.
  </p>
  {#if error}
    <div class="notice error" role="alert">{error}</div>
  {:else if series}
    <SimulationChart {series} simulationId={RUN_ID} bind:marks />
  {:else}
    <div class="notice">Loading series…</div>
  {/if}
</main>

<style>
  .harness {
    padding: 1.25rem;
    display: grid;
    gap: 0.8rem;
    max-width: 76rem;
    margin: 0 auto;
  }
</style>
