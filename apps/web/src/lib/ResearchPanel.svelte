<script lang="ts">
  import { onMount } from "svelte";
  import { errorText } from "./api";
  import {
    canCreate,
    type CreateImprovement,
    type ImprovementDecision,
    type ImprovementPartition,
    type ImprovementTier,
  } from "./research";
  import {
    compare,
    create,
    decide,
    load,
    research,
    reset,
    select,
    showBaseline,
    submit,
  } from "./research.svelte";

  const view = $derived(research());

  let tier = $state<ImprovementTier>("model");
  let hypothesis = $state("");
  let partition = $state<ImprovementPartition>("model_dev");
  let dataVersion = $state("");
  let configId = $state("");
  let criteriaId = $state("");
  let greenCriteria = $state('{"return_target":0.1}');
  let baseline = $state("");
  let codeCommit = $state("");
  let seed = $state(0);
  let analytics = $state(true);
  let decision = $state<ImprovementDecision>("rejected");
  let note = $state("");
  let baselineTier = $state<ImprovementTier>("model");

  const partitions: Record<ImprovementTier, ImprovementPartition[]> = {
    model: ["model_dev", "model_validation"],
    strategy: ["strategy_dev", "strategy_validation"],
    portfolio: ["portfolio_dev", "portfolio_holdout"],
  };
  const tiers: ImprovementTier[] = ["model", "strategy", "portfolio"];
  const decisions: ImprovementDecision[] = [
    "accepted",
    "rejected",
    "no-edge",
    "invalidated",
    "new-hypothesis",
  ];

  onMount(() => {
    void load();
    return () => reset();
  });

  function parsedGreenCriteria(): Record<string, unknown> | null {
    try {
      const value = JSON.parse(greenCriteria) as unknown;
      return value !== null && typeof value === "object" && !Array.isArray(value)
        ? (value as Record<string, unknown>)
        : null;
    } catch {
      return null;
    }
  }

  function draft(): CreateImprovement | null {
    const criteria = parsedGreenCriteria();
    if (criteria === null) {
      return null;
    }
    return {
      tier,
      hypothesis,
      partition,
      data_version: dataVersion,
      config_id: configId,
      criteria_id: criteriaId.trim() === "" ? null : criteriaId.trim(),
      green_criteria: criteria,
      baseline: baseline.trim() === "" ? null : baseline.trim(),
      code_commit: codeCommit,
      seed,
      analytics,
    };
  }

  const validDraft = $derived.by(() => {
    const value = draft();
    return value !== null && canCreate(value);
  });

  async function createRun(event: SubmitEvent) {
    event.preventDefault();
    const value = draft();
    if (value === null || !canCreate(value)) {
      return;
    }
    const created = await create(value);
    if (created) {
      hypothesis = "";
      dataVersion = "";
      configId = "";
      criteriaId = "";
      baseline = "";
      codeCommit = "";
    }
  }

  function open(id: string) {
    void select(id);
  }

  function chooseTier(next: ImprovementTier) {
    tier = next;
    partition = partitions[next][0]!;
  }
</script>

<section class="research">
  <header class="intro">
    <div>
      <h2>Research</h2>
      <p class="muted">
        Pre-registration, coupled submission, published-number comparison, and
        the explicit human decision.
      </p>
    </div>
    <button class="ghost" onclick={() => void load()}>Refresh</button>
  </header>

  {#if view.error}
    <p class="error" role="alert">{errorText(view.error)}</p>
  {/if}

  <p class="rule">
    A research run is a coupled run. The config must carry a strategy; a
    model-tier run still freezes strategy and portfolio.
  </p>
  <p class="rule">
    An accepted decision needs a succeeded pipeline run and canonical digest.
    Rejected, no-edge, new-hypothesis, and invalidated decisions may be recorded
    without a digest.
  </p>
  <p class="rule">
    Seeded historical baselines have no pipeline run, so their comparison values
    are unavailable. A stage artifact is never a pipeline run or a baseline.
  </p>

  <div class="grid">
    <form data-testid="research-create" onsubmit={createRun}>
      <h3>Create pre-registration</h3>
      <label>
        Tier
        <select bind:value={tier} onchange={() => chooseTier(tier)}>
          {#each tiers as item (item)}
            <option value={item}>{item}</option>
          {/each}
        </select>
      </label>
      <label>
        Hypothesis
        <textarea bind:value={hypothesis} rows="3" placeholder="One falsifiable sentence"></textarea>
      </label>
      <label>
        Partition
        <select bind:value={partition}>
          {#each partitions[tier] as item (item)}
            <option value={item}>{item}</option>
          {/each}
        </select>
      </label>
      <label>
        Data version
        <input bind:value={dataVersion} placeholder="sha256:…" />
      </label>
      <label>
        Config id
        <input bind:value={configId} placeholder="cfg_…" />
      </label>
      <label>
        Criteria id (strategy/portfolio)
        <input bind:value={criteriaId} placeholder="criteria_…" />
      </label>
      <label>
        Green criteria
        <textarea bind:value={greenCriteria} rows="3"></textarea>
      </label>
      <label>
        Baseline
        <input bind:value={baseline} placeholder="optional" />
      </label>
      <label>
        Code commit
        <input bind:value={codeCommit} placeholder="commit" />
      </label>
      <div class="row">
        <label>
          Seed
          <input bind:value={seed} type="number" min="0" step="1" />
        </label>
        <label class="check">
          <input bind:checked={analytics} type="checkbox" />
          Analytics
        </label>
      </div>
      <button type="submit" disabled={!validDraft || view.creating}>
        {view.creating ? "Creating…" : "Create pre-registration"}
      </button>
    </form>

    <div class="list">
      <h3>Improvement runs</h3>
      <table data-testid="research-table">
        <thead>
          <tr>
            <th>improvement_id</th>
            <th>tier</th>
            <th>state</th>
            <th>decision</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each view.items as item (item.improvement_id)}
            <tr>
              <td class="mono">{item.improvement_id}</td>
              <td>{item.tier}</td>
              <td>{item.state}</td>
              <td>{item.decision ?? "—"}</td>
              <td><button class="ghost" onclick={() => open(item.improvement_id)}>Open</button></td>
            </tr>
          {:else}
            <tr><td colspan="5" class="muted">No research runs.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  {#if view.detail}
    <article data-testid="research-detail" class="card">
      <header class="row">
        <div>
          <h3>{view.detail.improvement_id}</h3>
          <p class="muted">{view.detail.manifest.hypothesis}</p>
        </div>
        <button
          data-testid="research-submit"
          disabled={view.detail.state !== "created" || view.submitting}
          onclick={() => void submit(view.detail!.improvement_id)}
        >
          {view.submitting ? "Submitting…" : "Submit pipeline run"}
        </button>
      </header>
      <dl>
        <dt>state</dt><dd>{view.detail.state}</dd>
        <dt>decision</dt><dd>{view.detail.decision ?? "—"}</dd>
        <dt>pipeline_run_id</dt><dd class="mono">{view.detail.pipeline_run_id ?? "—"}</dd>
        <dt>canonical_digest</dt><dd class="mono">{view.detail.canonical_digest ?? "—"}</dd>
        <dt>created_at</dt><dd>{view.detail.created_at}</dd>
        <dt>submitted_at</dt><dd>{view.detail.submitted_at ?? "—"}</dd>
        <dt>decided_at</dt><dd>{view.detail.decided_at ?? "—"}</dd>
        <dt>partition</dt><dd>{view.detail.manifest.partition}</dd>
        <dt>changed</dt><dd>{view.detail.manifest.changed}</dd>
        <dt>frozen</dt><dd>{view.detail.manifest.frozen.join(", ")}</dd>
        <dt>data_version</dt><dd class="mono">{view.detail.manifest.data_version}</dd>
        <dt>config_id</dt><dd>{view.detail.manifest.config_id}</dd>
        <dt>criteria_id</dt><dd>{view.detail.manifest.criteria_id ?? "—"}</dd>
        <dt>baseline</dt><dd>{view.detail.manifest.baseline ?? "—"}</dd>
        <dt>decision_note</dt><dd>{view.detail.decision_note ?? "—"}</dd>
      </dl>

      <div class="row">
        <button class="ghost" onclick={() => void compare(view.detail!.improvement_id)}>
          Compare with baseline
        </button>
      </div>
      <table data-testid="research-compare">
        <thead>
          <tr><th>metric</th><th>candidate</th><th>baseline</th><th>delta</th></tr>
        </thead>
        <tbody>
          {#each view.comparison?.rows ?? [] as row (row.metric)}
            <tr>
              <td>{row.metric}</td>
              <td>{row.candidate ?? "—"}</td>
              <td>{row.baseline ?? "—"}</td>
              <td>{row.delta ?? "—"}</td>
            </tr>
          {:else}
            <tr><td colspan="4" class="muted">No comparison loaded.</td></tr>
          {/each}
        </tbody>
      </table>
      {#if view.comparison?.note}
        <p class="muted">{view.comparison.note}</p>
      {/if}

      <div data-testid="research-decide" class="decision">
        <h4>Human decision</h4>
        <p class="muted">
          This is an explicit human action. It is never submitted or accepted
          automatically.
        </p>
        <label>
          Decision
          <select bind:value={decision}>
            {#each decisions as item (item)}
              <option value={item}>{item}</option>
            {/each}
          </select>
        </label>
        <label>
          Note
          <input bind:value={note} placeholder="required by the server" />
        </label>
        <button
          disabled={view.deciding || note.trim() === ""}
          onclick={() => void decide(view.detail!.improvement_id, decision, note)}
        >
          {view.deciding ? "Recording…" : "Record decision"}
        </button>
      </div>
    </article>
  {/if}

  <div class="grid">
    <section>
      <h3>Accepted baselines</h3>
      <table data-testid="baseline-table">
        <thead>
          <tr><th>baseline_id</th><th>tier</th><th>accepted_on</th><th></th></tr>
        </thead>
        <tbody>
          {#each view.baselines as row (row.baseline_id)}
            <tr>
              <td class="mono">{row.baseline_id}</td>
              <td>{row.tier}</td>
              <td>{row.accepted_on}</td>
              <td>
                <button class="ghost" onclick={() => { baselineTier = row.tier; void showBaseline(row.tier); }}>
                  Show {row.tier}
                </button>
              </td>
            </tr>
          {:else}
            <tr><td colspan="4" class="muted">No accepted baselines.</td></tr>
          {/each}
        </tbody>
      </table>
      <div class="row">
        <label>
          Baseline tier
          <select bind:value={baselineTier}>
            {#each tiers as item (item)}
              <option value={item}>{item}</option>
            {/each}
          </select>
        </label>
        <button class="ghost" onclick={() => void showBaseline(baselineTier)}>Show baseline</button>
      </div>
      {#if view.baseline}
        <div data-testid="baseline-detail" class="card small">
          <strong>{view.baseline.baseline_id}</strong>
          <p>{view.baseline.scope}</p>
          <p class="muted">
            {view.baseline.accepted_by} · {view.baseline.accepted_on} · run
            {view.baseline.run_id ?? "—"}
          </p>
        </div>
      {/if}
    </section>

    <section>
      <h3>Invalidation tombstones</h3>
      <ul data-testid="invalidation-list" class="invalidation">
        {#each view.invalidations as row (row.invalidation_id)}
          <li>
            <strong>{row.invalidation_id}</strong>
            <div class="muted">{row.date} · {row.removed}</div>
            <div>{row.reason}</div>
          </li>
        {:else}
          <li class="muted">No invalidation tombstones.</li>
        {/each}
      </ul>
    </section>
  </div>
</section>

<style>
  .research { display: grid; gap: 1rem; }
  .intro { display: flex; justify-content: space-between; align-items: start; gap: 1rem; }
  h2, h3, h4 { margin: 0; }
  .grid { display: grid; grid-template-columns: minmax(300px, 0.8fr) minmax(0, 1.2fr); gap: 1rem; }
  .row { display: flex; gap: 0.75rem; align-items: end; flex-wrap: wrap; }
  .card { background: var(--panel); border: 1px solid var(--line); border-radius: 12px; padding: 1rem; }
  .card.small { margin-top: 0.75rem; }
  form, .list, section { background: var(--panel); border: 1px solid var(--line); border-radius: 12px; padding: 1rem; }
  label { display: grid; gap: 0.25rem; margin-bottom: 0.6rem; color: var(--muted); font-size: 0.9rem; }
  label.check { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0; }
  input, select, textarea { background: #0d1219; color: var(--text); border: 1px solid var(--line); border-radius: 8px; padding: 0.45rem 0.55rem; font: inherit; }
  textarea { resize: vertical; }
  table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
  th, td { text-align: left; border-bottom: 1px solid var(--line); padding: 0.4rem; vertical-align: top; }
  dt { color: var(--muted); font-size: 0.85rem; }
  dd { margin: 0 0 0.35rem; }
  dl { display: grid; grid-template-columns: 10rem minmax(0, 1fr); gap: 0.1rem 0.75rem; }
  .mono { font-family: ui-monospace, SFMono-Regular, monospace; overflow-wrap: anywhere; }
  .muted { color: var(--muted); }
  .error { color: #ff8a80; }
  .rule { margin: 0; padding: 0.6rem 0.8rem; background: #0d1219; border-left: 3px solid var(--accent); border-radius: 6px; }
  .decision { margin-top: 0.75rem; border-top: 1px solid var(--line); padding-top: 0.75rem; }
  .invalidation { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.6rem; }
  .invalidation li { border-bottom: 1px solid var(--line); padding-bottom: 0.5rem; }
  .ghost { background: transparent; border-color: var(--line); }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
</style>
