<script lang="ts">
  import { onMount } from "svelte";
  import { connection } from "./connection.svelte";
  import { shortId } from "./runs";
  import type {
    SimulationRun,
    SimulationStatus,
    SimulationWindow,
  } from "./simulation";
  import { countMarks, sortMarks } from "./simulationReview";
  import SimulationMarksTable from "./SimulationMarksTable.svelte";
  import SimulationCheckPanel from "./SimulationCheckPanel.svelte";
  import SimulationChart from "./SimulationChart.svelte";
  import SimulationDecisionPanel from "./SimulationDecisionPanel.svelte";
  import {
    openRun,
    openWindow,
    refreshRun,
    runOracle,
    setMarks,
    simulationRun,
  } from "./simulationRun.svelte";

  let { simId, onBack }: { simId: string; onBack: () => void } = $props();

  const view = $derived(simulationRun());
  const run: SimulationRun | null = $derived(view.run);
  const runLoading = $derived(view.runLoading);
  const runError = $derived(view.runError);
  const windows: SimulationWindow[] = $derived(view.windows);
  const windowsLoading = $derived(view.windowsLoading);
  const windowsError = $derived(view.windowsError);
  const chart = $derived(view.chart);
  const series = $derived(view.series);
  const seriesLoading = $derived(view.seriesLoading);
  const seriesError = $derived(view.seriesError);
  const marks = $derived(view.marks);
  const marksLoading = $derived(view.marksLoading);
  const marksError = $derived(view.marksError);
  const oracleBusy = $derived(view.oracleBusy);
  const actionError = $derived(view.actionError);
  const actionStatus = $derived(view.actionStatus);

  const link = $derived(connection());
  const locked = $derived(run?.status === "decided" || run?.decision != null);
  const ordered = $derived(sortMarks(marks));
  const human = $derived(countMarks(marks, "human"));
  const oracle = $derived(countMarks(marks, "oracle"));

  onMount(() => {
    void openRun(simId);
  });

  function statusBadge(status: SimulationStatus): string {
    switch (status) {
      case "draft":
        return "";
      case "marking":
        return "warn";
      case "reviewed":
      case "checked":
      case "decided":
        return "ok";
    }
  }

  function utc(iso: string | undefined): string {
    if (iso === undefined) {
      return "—";
    }
    return iso.replace("T", " ").replace(".000Z", "Z");
  }

  function windowMarkCount(index: number): number {
    return marks.filter((mark) => mark.chart === index).length;
  }
</script>

<section class="stack" data-testid="simulation-run">
  <div class="row">
    <button onclick={onBack} aria-label="Back to simulation runs" data-testid="sim-back">
      Back
    </button>
    <h2>Simulation run</h2>
    <span class="mono" title={simId}>{shortId(simId)}</span>
    {#if run}
      <span class="badge {statusBadge(run.status)}" data-testid="sim-status">
        {run.status}
      </span>
    {/if}
  </div>

  {#if link.state === "unreachable"}
    <div class="notice error" role="alert" data-testid="simulation-run-disconnected">
      Disconnected. The simulation run is not shown.
    </div>
  {:else if runLoading && run === null}
    <div class="notice">Loading simulation run…</div>
  {:else if runError}
    <div class="notice error" role="alert" data-testid="sim-run-error">{runError}</div>
  {:else if run === null}
    <div class="notice">The simulation run is unavailable.</div>
  {:else}
    <table class="meta" data-testid="sim-run-meta">
      <tbody>
        <tr><th>Premise</th><td>{run.premise}</td></tr>
        {#if run.bundle}
          <tr><th>Bundle</th><td class="mono wrap">{run.bundle}</td></tr>
        {/if}
        {#if run.inputs}
          <tr><th>Inputs</th><td>{run.inputs}</td></tr>
        {/if}
        {#if run.inputs_digest}
          <tr><th>Inputs digest</th><td class="mono wrap">{run.inputs_digest}</td></tr>
        {/if}
        {#if run.seed !== undefined}
          <tr><th>Seed</th><td>{run.seed}</td></tr>
        {/if}
        {#if run.criteria}
          <tr><th>Criteria</th><td class="wrap">{run.criteria}</td></tr>
        {/if}
      </tbody>
    </table>

    {#if locked}
      <div class="notice lock" role="status" data-testid="sim-lock">
        Marking is closed. This run has a decision.
        {#if run.decision}
          <span class="muted">Recorded {run.decision.decision}.</span>
        {/if}
      </div>
    {/if}

    {#if actionError}
      <div class="notice error" role="alert" data-testid="sim-action-error">
        {actionError}
      </div>
    {/if}
    {#if actionStatus}
      <p class="muted small" role="status" data-testid="sim-action-status">
        {actionStatus}
      </p>
    {/if}

    <div class="row">
      <h3>Windows</h3>
      <span class="badge">{windows.length}</span>
      <span class="muted small">The server selects the windows from the run seed.</span>
    </div>

    {#if windowsError}
      <div class="notice error" role="alert" data-testid="sim-windows-error">
        {windowsError}
      </div>
    {:else if windowsLoading && windows.length === 0}
      <div class="notice">Selecting windows…</div>
    {:else if windows.length === 0}
      <div class="notice" data-testid="sim-no-windows">
        No windows selected. The server selects windows from the run seed; reload the
        run to retry.
      </div>
    {:else}
      <div class="toolbar" data-testid="sim-windows">
        {#each windows as window (window.index)}
          <button
            class:selected={chart === window.index}
            aria-pressed={chart === window.index}
            onclick={() => void openWindow(simId, window.index)}
            data-testid={`sim-window-${window.index}`}
          >
            W{window.index} · {window.start}–{window.end}
            <span class="muted small">{windowMarkCount(window.index)} marks</span>
          </button>
        {/each}
      </div>

      {#if series}
        <SimulationChart
          {series}
          simulationId={simId}
          {marks}
          {locked}
          onMarksChanged={(next) => setMarks(next)}
          onMarkPosted={() => void refreshRun(simId)}
        />
      {:else if seriesLoading}
        <div class="notice">Loading window…</div>
      {:else if seriesError}
        <div class="notice error" role="alert" data-testid="sim-series-error">
          {seriesError}
        </div>
      {:else}
        <div class="notice">Select a window to draw it.</div>
      {/if}
    {/if}

    <SimulationMarksTable marks={marks} loading={marksLoading} error={marksError} />

    <section class="review" data-testid="sim-review">
      <div class="row">
        <h3>Review</h3>
        <button
          onclick={() => void runOracle(simId)}
          disabled={oracleBusy || locked}
          data-testid="sim-oracle"
        >
          {oracleBusy ? "Running oracle…" : "Run oracle"}
        </button>
        <span class="muted small">
          Compare the human and oracle sets here. A disagreement is a finding, not
          a defect to hide.
        </span>
      </div>

      {#if marksLoading && ordered.length === 0}
        <div class="notice">Loading marks…</div>
      {:else}
        <div class="review-grid">
          <div class="review-card">
            <div class="row">
              <strong>Human</strong>
              <span class="badge ok">{human.total}</span>
            </div>
            <table class="counts">
              <tbody>
                <tr><th>Total</th><td data-testid="sim-review-human-total">{human.total}</td></tr>
                <tr><th>Good</th><td>{human.good}</td></tr>
                <tr><th>Bad</th><td>{human.bad}</td></tr>
                <tr><th>Neutral</th><td>{human.neutral}</td></tr>
                <tr><th>Long</th><td>{human.long}</td></tr>
                <tr><th>Short</th><td>{human.short}</td></tr>
                <tr><th>Entry</th><td>{human.entry}</td></tr>
                <tr><th>Exit</th><td>{human.exit}</td></tr>
              </tbody>
            </table>
          </div>

          <div class="review-card">
            <div class="row">
              <strong>Oracle</strong>
              <span class="badge warn">{oracle.total}</span>
            </div>
            <table class="counts">
              <tbody>
                <tr><th>Total</th><td data-testid="sim-review-oracle-total">{oracle.total}</td></tr>
                <tr><th>Good</th><td>{oracle.good}</td></tr>
                <tr><th>Bad</th><td>{oracle.bad}</td></tr>
                <tr><th>Neutral</th><td>{oracle.neutral}</td></tr>
                <tr><th>Long</th><td>{oracle.long}</td></tr>
                <tr><th>Short</th><td>{oracle.short}</td></tr>
                <tr><th>Entry</th><td>{oracle.entry}</td></tr>
                <tr><th>Exit</th><td>{oracle.exit}</td></tr>
              </tbody>
            </table>
          </div>
        </div>

        {#if human.total === 0 && oracle.total === 0}
          <div class="notice" data-testid="sim-review-empty">No marks to review.</div>
        {:else if oracle.total === 0}
          <div class="notice" data-testid="sim-review-waiting">
            Run the oracle to compare its marks with the human set.
          </div>
        {/if}
      {/if}
    </section>

    <SimulationCheckPanel {simId} />

    <SimulationDecisionPanel
      {simId}
      decidedAt={run.decision?.decided_at_utc
        ? utc(run.decision.decided_at_utc)
        : undefined}
    />
  {/if}
</section>

<style>
  section {
    max-width: 84rem;
    gap: 0.8rem;
  }

  .meta {
    max-width: 84rem;
  }

  .wrap {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .toolbar button {
    font-size: 0.8rem;
    padding: 0.35rem 0.55rem;
  }

  .toolbar button.selected {
    border-color: var(--accent);
    color: var(--accent);
  }

  .review {
    border-top: 1px solid var(--line);
    padding-top: 0.8rem;
  }

  .review-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.8rem;
  }

  .review-card {
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
    padding: 0.6rem 0.75rem;
  }

  .counts th,
  .counts td {
    padding: 0.2rem 0.4rem;
    border-bottom: none;
  }

  .lock {
    border-color: var(--warn);
    color: var(--warn);
  }

  @media (max-width: 900px) {
    .review-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
