<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import { markClassLabel } from "./chart/markClasses";
  import type { ChartMark, ChartSeries } from "./chart/types";
  import { connection } from "./connection.svelte";
  import { shortId } from "./runs";
  import {
    fetchMarks,
    fetchRun,
    fetchSeries,
    postCheck,
    postDecision,
    postOracle,
    selectWindows,
    type SimulationCheck,
    type SimulationDecisionKind,
    type SimulationRun,
    type SimulationStatus,
    type SimulationWindow,
  } from "./simulation";
  import { countMarks, sortMarks } from "./simulationReview";
  import SimulationChart from "./SimulationChart.svelte";

  let { simId, onBack }: { simId: string; onBack: () => void } = $props();

  let run = $state<SimulationRun | null>(null);
  let runLoading = $state(true);
  let runError = $state<string | null>(null);

  let windows = $state<SimulationWindow[]>([]);
  let windowsLoading = $state(false);
  let windowsError = $state<string | null>(null);
  let chart = $state<number | null>(null);

  let series = $state<ChartSeries | null>(null);
  let seriesLoading = $state(false);
  let seriesError = $state<string | null>(null);
  let windowRequest = 0;

  let marks = $state<ChartMark[]>([]);
  let marksLoading = $state(false);
  let marksError = $state<string | null>(null);
  let marksRequest = 0;

  let oracleBusy = $state(false);
  let checkBusy = $state(false);
  let checkResult = $state<SimulationCheck | null>(null);
  let checkError = $state<string | null>(null);
  let decisionKind = $state<SimulationDecisionKind>("test");
  let decisionNote = $state("");
  let decisionBusy = $state(false);
  let decisionError = $state<string | null>(null);

  let actionError = $state<string | null>(null);
  let actionStatus = $state<string | null>(null);

  const link = $derived(connection());
  const locked = $derived(run?.status === "decided" || run?.decision != null);
  const ordered = $derived(sortMarks(marks));
  const human = $derived(countMarks(marks, "human"));
  const oracle = $derived(countMarks(marks, "oracle"));

  const FAILURE_TEXT: Readonly<Record<string, string>> = {
    too_few_events: "too few filtered events",
    mean_not_positive: "the filtered mean is not positive",
    hit_rate_low: "the filtered hit rate is too low",
    control_mean_not_beaten: "the control mean is not beaten",
    control_hit_rate_not_beaten: "the control hit rate is not beaten",
    unstable: "the direction does not hold across sub-periods",
  };

  onMount(() => {
    void open();
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

  async function open(): Promise<void> {
    runLoading = true;
    runError = null;
    try {
      run = await fetchRun(simId);
    } catch (error) {
      runLoading = false;
      runError = errorText(error);
      return;
    }
    runLoading = false;
    await Promise.all([loadWindows(), loadMarks()]);
  }

  async function loadWindows(): Promise<void> {
    windowsLoading = true;
    windowsError = null;
    try {
      const found = await selectWindows(simId);
      windows = found;
      if (found.length === 0) {
        chart = null;
        series = null;
        return;
      }
      const preferred =
        chart !== null && found.some((window) => window.index === chart)
          ? chart
          : found[0].index;
      await openWindow(preferred);
    } catch (error) {
      windowsError = errorText(error);
    } finally {
      windowsLoading = false;
    }
  }

  async function openWindow(index: number): Promise<void> {
    const request = (windowRequest += 1);
    chart = index;
    series = null;
    seriesError = null;
    seriesLoading = true;
    try {
      const next = await fetchSeries(simId, index);
      if (request === windowRequest) {
        series = next;
      }
    } catch (error) {
      if (request === windowRequest) {
        seriesError = errorText(error);
      }
    } finally {
      if (request === windowRequest) {
        seriesLoading = false;
      }
    }
  }

  async function loadMarks(): Promise<void> {
    const request = (marksRequest += 1);
    marksLoading = true;
    marksError = null;
    try {
      const next = await fetchMarks(simId);
      if (request === marksRequest) {
        marks = next;
      }
    } catch (error) {
      if (request === marksRequest) {
        marks = [];
        marksError = errorText(error);
      }
    } finally {
      if (request === marksRequest) {
        marksLoading = false;
      }
    }
  }

  async function refreshRun(): Promise<void> {
    try {
      run = await fetchRun(simId);
    } catch {
      // The action already succeeded against the server; keep the confirmed run.
    }
  }

  async function runOracle(): Promise<void> {
    if (oracleBusy || locked) {
      return;
    }
    oracleBusy = true;
    actionError = null;
    actionStatus = null;
    try {
      await postOracle(simId);
      await Promise.all([loadMarks(), refreshRun()]);
      actionStatus = "Oracle marks recorded.";
    } catch (error) {
      actionError = errorText(error);
    } finally {
      oracleBusy = false;
    }
  }

  async function runCheck(): Promise<void> {
    if (checkBusy || locked) {
      return;
    }
    checkBusy = true;
    checkResult = null;
    checkError = null;
    actionError = null;
    actionStatus = null;
    try {
      checkResult = await postCheck(simId);
      await refreshRun();
      actionStatus = checkResult.passed ? "The check passed." : "The check failed.";
    } catch (error) {
      checkError = errorText(error);
    } finally {
      checkBusy = false;
    }
  }

  async function submitDecision(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (decisionBusy || locked) {
      return;
    }
    const note = decisionNote.trim();
    if (note === "") {
      decisionError = "A decision needs a note.";
      return;
    }
    decisionBusy = true;
    decisionError = null;
    actionError = null;
    actionStatus = null;
    try {
      const record = await postDecision(simId, decisionKind, note);
      if (run !== null) {
        run = { ...run, status: "decided", decision: record };
      }
      decisionNote = "";
      actionStatus = "Decision recorded. Marking is closed.";
    } catch (error) {
      decisionError = errorText(error);
    } finally {
      decisionBusy = false;
    }
  }

  function utc(iso: string | undefined): string {
    if (iso === undefined) {
      return "—";
    }
    return iso.replace("T", " ").replace(".000Z", "Z");
  }

  function percent(value: number): string {
    return `${(value * 100).toFixed(2)}%`;
  }

  function decimal(value: number): string {
    return value.toFixed(4);
  }

  function failureText(code: string): string {
    return FAILURE_TEXT[code] ?? code;
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
            onclick={() => void openWindow(window.index)}
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
          bind:marks
          {locked}
          onMarksChange={() => void refreshRun()}
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

    <div class="row">
      <h3>Marks</h3>
      <span class="badge">{marks.length}</span>
      <span class="muted small">Human and oracle rows are kept distinct.</span>
    </div>

    {#if marksError}
      <div class="notice error" role="alert" data-testid="sim-marks-error">
        {marksError}
      </div>
    {:else if marksLoading && ordered.length === 0}
      <div class="notice">Loading marks…</div>
    {:else if ordered.length === 0}
      <div class="notice" data-testid="sim-marks-empty">
        No marks yet. Point at a bar and press 1–8.
      </div>
    {:else}
      <table data-testid="sim-marks">
        <thead>
          <tr>
            <th>Mark</th>
            <th>Origin</th>
            <th>Window</th>
            <th>Bar</th>
            <th>Class</th>
            <th>Note</th>
            <th>Recorded (UTC)</th>
          </tr>
        </thead>
        <tbody>
          {#each ordered as mark (mark.mark_id)}
            <tr class="mark-row {mark.origin}" data-testid={`sim-mark-${mark.mark_id}`}>
              <td class="mono">{mark.mark_id}</td>
              <td>
                <span
                  class="badge {mark.origin === 'human' ? 'ok' : 'warn'}"
                  data-origin={mark.origin}
                >
                  {mark.origin}
                </span>
              </td>
              <td>{mark.chart}</td>
              <td>{mark.bar}</td>
              <td>{markClassLabel(mark)}</td>
              <td class="note">{mark.note === "" ? "—" : mark.note}</td>
              <td class="mono">{utc(mark.marked_time_utc)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <section class="review" data-testid="sim-review">
      <div class="row">
        <h3>Review</h3>
        <button
          onclick={() => void runOracle()}
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

    <section class="check" data-testid="sim-check">
      <div class="row">
        <h3>Check</h3>
        <button
          onclick={() => void runCheck()}
          disabled={checkBusy || locked}
          data-testid="sim-check-run"
        >
          {checkBusy ? "Checking…" : "Run check"}
        </button>
        {#if locked}
          <span class="muted small">A decided run is closed.</span>
        {/if}
      </div>

      {#if checkError}
        <div class="notice error" role="alert" data-testid="sim-check-error">
          {checkError}
        </div>
      {:else if checkBusy}
        <div class="notice" data-testid="sim-check-busy">Running the check…</div>
      {:else if checkResult}
        <table data-testid="sim-check-table">
          <tbody>
            <tr><th>Filtered events</th><td>{checkResult.events}</td></tr>
            <tr><th>Mean net (R)</th><td>{decimal(checkResult.mean_net)}</td></tr>
            <tr><th>Hit rate</th><td>{percent(checkResult.hit_rate)}</td></tr>
            <tr><th>Control events</th><td>{checkResult.control}</td></tr>
            <tr><th>Control mean (R)</th><td>{decimal(checkResult.control_mean_net)}</td></tr>
            <tr><th>Control hit rate</th><td>{percent(checkResult.control_hit_rate)}</td></tr>
            <tr><th>Positive half-years</th><td>{checkResult.positive_half_years}</td></tr>
          </tbody>
        </table>
        <div
          class="notice"
          class:error={!checkResult.passed}
          role="status"
          data-testid="sim-check-verdict"
        >
          {checkResult.passed ? "passed" : "failed"}
        </div>
        {#if checkResult.failures.length > 0}
          <ul data-testid="sim-check-failures">
            {#each checkResult.failures as failure (failure)}
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

    <section class="decision" data-testid="sim-decision">
      <h3>Decision</h3>

      {#if run.decision}
        <div class="notice" data-testid="sim-decision-record">
          <div class="row">
            <strong>{run.decision.decision}</strong>
            {#if run.decision.decided_at_utc}
              <span class="muted small">{utc(run.decision.decided_at_utc)}</span>
            {/if}
          </div>
          <p>{run.decision.note}</p>
          <p class="muted small">Marking is closed for this run.</p>
        </div>
      {:else if locked}
        <div class="notice" data-testid="sim-decision-record">
          This run is decided; marking is closed.
        </div>
      {:else}
        <form class="stack" onsubmit={(event) => void submitDecision(event)}>
          <fieldset>
            <legend class="muted small">Outcome</legend>
            <div class="row options">
              <label class="row">
                <input
                  type="radio"
                  bind:group={decisionKind}
                  value="test"
                  data-testid="sim-decision-test"
                />
                Test
              </label>
              <label class="row">
                <input
                  type="radio"
                  bind:group={decisionKind}
                  value="refine"
                  data-testid="sim-decision-refine"
                />
                Refine
              </label>
              <label class="row">
                <input
                  type="radio"
                  bind:group={decisionKind}
                  value="stop"
                  data-testid="sim-decision-stop"
                />
                Stop
              </label>
            </div>
          </fieldset>

          <label>
            <span class="muted small">Note</span>
            <textarea
              bind:value={decisionNote}
              rows="2"
              placeholder="Why this outcome; the candidate rule, if any."
              data-testid="sim-decision-note"
            ></textarea>
          </label>

          {#if decisionError}
            <div class="notice error" role="alert" data-testid="sim-decision-error">
              {decisionError}
            </div>
          {/if}

          <div class="row">
            <button
              type="submit"
              disabled={decisionBusy || decisionNote.trim() === ""}
              data-testid="sim-decision-submit"
            >
              {decisionBusy ? "Recording…" : "Record decision"}
            </button>
            <span class="muted small">Recording a decision locks marking.</span>
          </div>
        </form>
      {/if}
    </section>
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

  .note {
    max-width: 28rem;
    overflow-wrap: anywhere;
  }

  .mark-row.oracle td:first-child {
    border-left: 3px solid var(--warn);
  }

  .mark-row.human td:first-child {
    border-left: 3px solid var(--ok);
  }

  .review,
  .check,
  .decision {
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

  .options {
    flex-wrap: wrap;
  }

  fieldset {
    border: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.3rem;
  }

  legend {
    padding: 0;
  }

  input,
  textarea {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.45rem 0.6rem;
  }

  input[type="radio"] {
    width: auto;
    accent-color: var(--accent);
  }

  textarea {
    resize: vertical;
    width: 100%;
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
