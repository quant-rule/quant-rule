<script lang="ts">
  import type { SimulationDecisionKind } from "./simulation";
  import { simulationRun, submitDecision } from "./simulationRun.svelte";

  let {
    simId,
    decidedAt,
  }: { simId: string; decidedAt: string | undefined } = $props();

  const view = $derived(simulationRun());
  const decision = $derived(view.run?.decision ?? null);
  const locked = $derived(
    view.run?.status === "decided" || decision !== null,
  );

  let decisionKind = $state<SimulationDecisionKind>("test");
  let decisionNote = $state("");

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (await submitDecision(simId, decisionKind, decisionNote)) {
      decisionNote = "";
    }
  }
</script>

<section class="decision" data-testid="sim-decision">
  <h3>Decision</h3>

  {#if decision}
    <div class="notice" data-testid="sim-decision-record">
      <div class="row">
        <strong>{decision.decision}</strong>
        {#if decidedAt}
          <span class="muted small">{decidedAt}</span>
        {/if}
      </div>
      <p>{decision.note}</p>
      <p class="muted small">Marking is closed for this run.</p>
    </div>
  {:else if locked}
    <div class="notice" data-testid="sim-decision-record">
      This run is decided; marking is closed.
    </div>
  {:else}
    <form class="stack" onsubmit={(event) => void submit(event)}>
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

      {#if view.decisionError}
        <div class="notice error" role="alert" data-testid="sim-decision-error">
          {view.decisionError}
        </div>
      {/if}

      <div class="row">
        <button
          type="submit"
          disabled={view.decisionBusy || decisionNote.trim() === ""}
          data-testid="sim-decision-submit"
        >
          {view.decisionBusy ? "Recording…" : "Record decision"}
        </button>
        <span class="muted small">Recording a decision locks marking.</span>
      </div>
    </form>
  {/if}
</section>

<style>
  .decision {
    border-top: 1px solid var(--line);
    padding-top: 0.8rem;
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
</style>
