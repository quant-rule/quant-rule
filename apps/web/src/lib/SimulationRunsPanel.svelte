<script lang="ts">
  import { onMount } from "svelte";
  import {
    connection,
    refresh as refreshConnection,
  } from "./connection.svelte";
  import { shortId } from "./runs";
  import {
    canCreate,
    type SimulationCreate,
    type SimulationStatus,
  } from "./simulation";
  import { create, load, simulations } from "./simulation.svelte";
  import { profiles } from "./profiles.svelte";
  import SimulationRunDetail from "./SimulationRunDetail.svelte";

  const view = $derived(simulations());
  const link = $derived(connection());
  const active = $derived(profiles().active);

  let selected = $state<string | null>(null);
  let premise = $state("");
  let bundle = $state("");
  let inputs = $state("");
  let seed = $state(0);
  let criteria = $state("");
  let formError = $state<string | null>(null);
  let scope = $state<string | null>(null);

  // A different profile must not keep the previous profile's open simulation.
  $effect(() => {
    if (scope === null) {
      scope = active;
      return;
    }
    if (scope !== active) {
      scope = active;
      selected = null;
    }
  });

  onMount(() => {
    void load();
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

  async function refreshAll(): Promise<void> {
    await refreshConnection();
    await load();
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (view.creating) {
      return;
    }
    const input: SimulationCreate = {
      premise: premise.trim(),
      bundle: bundle.trim(),
      inputs: inputs.trim(),
      seed,
      criteria: criteria.trim(),
    };
    if (!canCreate(input)) {
      formError =
        "A premise, bundle ref, inputs, non-negative seed, and criteria are required.";
      return;
    }
    formError = null;
    const simId = await create(input);
    if (simId !== null) {
      premise = "";
      bundle = "";
      inputs = "";
      seed = 0;
      criteria = "";
      selected = simId;
    }
  }

  function back(): void {
    selected = null;
    void load();
  }
</script>

{#if selected}
  <SimulationRunDetail simId={selected} onBack={back} />
{:else}
  <section class="stack" data-testid="simulation-runs">
    <div class="row">
      <h2>Simulation runs</h2>
      <span class="badge">{view.items.length} total</span>
      <button
        onclick={() => void refreshAll()}
        disabled={view.loading}
        data-testid="simulation-refresh"
      >
        Refresh
      </button>
    </div>

    {#if link.state === "unreachable"}
      <div class="notice error" role="alert" data-testid="simulation-disconnected">
        Disconnected. The simulation list is not shown.
      </div>
      <button onclick={() => void refreshAll()} data-testid="simulation-retry">
        Retry
      </button>
    {:else}
      {#if view.error}
        <div class="notice error" role="alert" data-testid="simulation-error">
          {view.error}
        </div>
      {/if}

      {#if view.loading && view.items.length === 0}
        <div class="notice">Loading simulation runs…</div>
      {:else if view.error === null && view.items.length === 0}
        <div class="notice" data-testid="simulation-empty">No simulation runs.</div>
      {:else if view.items.length > 0}
        <table data-testid="simulation-table" aria-busy={view.loading}>
          <thead>
            <tr>
              <th>Simulation</th>
              <th>Premise</th>
              <th>Status</th>
              <th>Windows</th>
              <th>Marked</th>
              <th>Marks</th>
              <th>Bundle</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each view.items as item (item.sim_id)}
              <tr>
                <td class="mono" title={item.sim_id}>{shortId(item.sim_id)}</td>
                <td class="premise">{item.premise}</td>
                <td>
                  <span class="badge {statusBadge(item.status)}">{item.status}</span>
                </td>
                <td>{item.windows}</td>
                <td>{item.marked_windows}</td>
                <td>{item.marks}</td>
                <td class="mono" title={item.bundle}>
                  {shortId(item.bundle)}
                </td>
                <td>
                  <button
                    aria-label={`Open ${item.sim_id}`}
                    onclick={() => (selected = item.sim_id)}
                  >
                    Open
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}

      <form
        class="stack create"
        onsubmit={(event) => void submit(event)}
        data-testid="simulation-create"
      >
        <div class="row">
          <h3>New simulation</h3>
          <span class="muted small">
            Premise, bundle, inputs, seed, and criteria are recorded before the
            first mark.
          </span>
        </div>

        <div class="form-grid">
          <label>
            <span class="muted small">Premise</span>
            <input
              bind:value={premise}
              placeholder="What pattern are we looking for?"
              data-testid="sim-create-premise"
            />
          </label>
          <label>
            <span class="muted small">Bundle ref</span>
            <input
              bind:value={bundle}
              placeholder="sha256:…"
              spellcheck="false"
              data-testid="sim-create-bundle"
            />
          </label>
          <label>
            <span class="muted small">Inputs</span>
            <input
              bind:value={inputs}
              placeholder="One or two registered events or readings"
              data-testid="sim-create-inputs"
            />
          </label>
          <label>
            <span class="muted small">Seed</span>
            <input
              type="number"
              min="0"
              step="1"
              bind:value={seed}
              data-testid="sim-create-seed"
            />
          </label>
        </div>

        <label>
          <span class="muted small">Criteria</span>
          <textarea
            bind:value={criteria}
            rows="2"
            placeholder="Mark criterion and check thresholds, recorded before the first mark."
            data-testid="sim-create-criteria"
          ></textarea>
        </label>

        {#if formError}
          <div class="notice error" role="alert" data-testid="sim-create-error">
            {formError}
          </div>
        {/if}

        <div class="row">
          <button
            type="submit"
            disabled={view.creating}
            data-testid="sim-create-submit"
          >
            {view.creating ? "Creating…" : "Create simulation"}
          </button>
          <span class="muted small">The server assigns the sim id.</span>
        </div>
      </form>
    {/if}
  </section>
{/if}

<style>
  section {
    max-width: 84rem;
    gap: 0.8rem;
  }

  .premise {
    max-width: 32rem;
  }

  .create {
    border-top: 1px solid var(--line);
    padding-top: 0.8rem;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.6rem;
  }

  label {
    display: grid;
    gap: 0.25rem;
  }

  input,
  textarea {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.45rem 0.6rem;
    width: 100%;
  }

  textarea {
    resize: vertical;
  }

  @media (max-width: 900px) {
    .form-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
