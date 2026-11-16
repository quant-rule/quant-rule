<script lang="ts">
  import { runCheck, simulationRun } from "./simulationRun.svelte";

  let { simId }: { simId: string } = $props();

  const view = $derived(simulationRun());
  const locked = $derived(
    view.run?.status === "decided" || view.run?.decision != null,
  );

  const FAILURE_TEXT: Readonly<Record<string, string>> = {
    too_few_events: "too few filtered events",
    mean_not_positive: "the filtered mean is not positive",
    hit_rate_low: "the filtered hit rate is too low",
    control_mean_not_beaten: "the control mean is not beaten",
    control_hit_rate_not_beaten: "the control hit rate is not beaten",
    unstable: "the direction does not hold across sub-periods",
  };

  function percent(value: number): string {
    return `${(value * 100).toFixed(2)}%`;
  }

  function decimal(value: number): string {
    return value.toFixed(4);
  }

  function failureText(code: string): string {
    return FAILURE_TEXT[code] ?? code;
  }
</script>

<section class="check" data-testid="sim-check">
  <div class="row">
    <h3>Check</h3>
    <button
      onclick={() => void runCheck(simId)}
      disabled={view.checkBusy || locked}
      data-testid="sim-check-run"
    >
      {view.checkBusy ? "Checking…" : "Run check"}
    </button>
    {#if locked}
      <span class="muted small">A decided run is closed.</span>
    {/if}
  </div>

  {#if view.checkError}
    <div class="notice error" role="alert" data-testid="sim-check-error">
      {view.checkError}
    </div>
  {:else if view.checkBusy}
    <div class="notice" data-testid="sim-check-busy">Running the check…</div>
  {:else if view.checkResult}
    <table data-testid="sim-check-table">
      <tbody>
        <tr><th>Filtered events</th><td>{view.checkResult.events}</td></tr>
        <tr><th>Mean net (R)</th><td>{decimal(view.checkResult.mean_net)}</td></tr>
        <tr><th>Hit rate</th><td>{percent(view.checkResult.hit_rate)}</td></tr>
        <tr><th>Control events</th><td>{view.checkResult.control}</td></tr>
        <tr>
          <th>Control mean (R)</th>
          <td>{decimal(view.checkResult.control_mean_net)}</td>
        </tr>
        <tr>
          <th>Control hit rate</th>
          <td>{percent(view.checkResult.control_hit_rate)}</td>
        </tr>
        <tr>
          <th>Positive half-years</th>
          <td>{view.checkResult.positive_half_years}</td>
        </tr>
      </tbody>
    </table>
    <div
      class="notice"
      class:error={!view.checkResult.passed}
      role="status"
      data-testid="sim-check-verdict"
    >
      {view.checkResult.passed ? "passed" : "failed"}
    </div>
    {#if view.checkResult.failures.length > 0}
      <ul data-testid="sim-check-failures">
        {#each view.checkResult.failures as failure (failure)}
          <li>{failureText(failure)}</li>
        {/each}
      </ul>
    {/if}
  {:else}
    <div class="notice" data-testid="sim-check-empty">
      No check yet. The check runs on a later, unmarked window.
    </div>
  {/if}
</section>

<style>
  .check {
    border-top: 1px solid var(--line);
    padding-top: 0.8rem;
  }
</style>
