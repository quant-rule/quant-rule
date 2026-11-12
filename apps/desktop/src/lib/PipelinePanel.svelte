<script lang="ts">
  import { call } from "./api";
  import { errorMessage } from "./errors";

  type Field = "path" | "config" | "model" | "strategy" | "decisions" | "fills" | "evidence" | "criteria" | "spec" | "out";

  type Stage = {
    id: string;
    label: string;
    command: string;
    fields: Field[];
    hint: string;
  };

  const stages: Stage[] = [
    { id: "candles", label: "Inspect candles", command: "inspect_candles", fields: ["path"], hint: "Strict M1 CSV source." },
    { id: "timeframes", label: "Inspect timeframes", command: "inspect_timeframes", fields: ["path"], hint: "Build the configured ladder." },
    { id: "events", label: "Compute events", command: "compute_events", fields: ["path"], hint: "Event batch over one source." },
    { id: "report", label: "Model report", command: "run_model_report", fields: ["config", "spec", "out"], hint: "Write the model bundle." },
    { id: "explain", label: "Explain strategy", command: "explain_strategy", fields: ["model", "strategy"], hint: "One decision and its evidence." },
    { id: "strategy", label: "Run strategy", command: "run_strategy", fields: ["model", "strategy", "out"], hint: "Write the decisions bundle." },
    { id: "execution", label: "Run execution", command: "run_execution", fields: ["decisions", "config", "out"], hint: "Write the fills bundle." },
    { id: "portfolio", label: "Replay portfolio", command: "replay_portfolio", fields: ["fills", "config", "out"], hint: "Write the portfolio bundle." },
    { id: "backtest", label: "Backtest", command: "run_backtest", fields: ["config", "strategy", "out"], hint: "Run the coupled stages." },
    { id: "qualify", label: "Qualify", command: "qualify", fields: ["evidence", "criteria", "out"], hint: "Evaluate human-owned criteria." },
    { id: "analytics", label: "Analytics", command: "evaluate_analytics", fields: ["evidence", "out"], hint: "Write descriptive metrics." },
  ];

  let { runPath = "" }: { runPath?: string } = $props();

  let selected = $state("report");
  let fields = $state<Record<Field, string>>({
    path: "",
    config: "",
    model: "",
    strategy: "",
    decisions: "",
    fills: "",
    evidence: "",
    criteria: "",
    spec: "",
    out: "",
  });
  let result = $state<unknown>(null);
  let busy = $state(false);
  let error = $state("");

  $effect(() => {
    if (runPath !== "") {
      fields.config = runPath;
    }
  });

  const stage = $derived(stages.find((item) => item.id === selected) ?? stages[0]);

  async function run() {
    busy = true;
    error = "";
    result = null;
    const args: Record<string, unknown> = {};
    for (const field of stage.fields) {
      const value = fields[field].trim();
      if (value !== "") {
        args[field] = value;
      }
    }
    try {
      switch (stage.id) {
        case "candles":
          result = await call<unknown>("inspect_candles", args);
          break;
        case "timeframes":
          result = await call<unknown>("inspect_timeframes", args);
          break;
        case "events":
          result = await call<unknown>("compute_events", args);
          break;
        case "report":
          result = await call<unknown>("run_model_report", args);
          break;
        case "explain":
          result = await call<unknown>("explain_strategy", args);
          break;
        case "strategy":
          result = await call<unknown>("run_strategy", args);
          break;
        case "execution":
          result = await call<unknown>("run_execution", args);
          break;
        case "portfolio":
          result = await call<unknown>("replay_portfolio", args);
          break;
        case "backtest":
          result = await call<unknown>("run_backtest", args);
          break;
        case "qualify":
          result = await call<unknown>("qualify", args);
          break;
        case "analytics":
          result = await call<unknown>("evaluate_analytics", args);
          break;
      }
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="split">
  <div class="card">
    <h2>Pipeline</h2>
    <p class="muted small">
      Every stage calls the shared application service. The desktop does not
      parse a configuration or compute a model value.
    </p>
    <div class="list">
      {#each stages as item (item.id)}
        <button class:active={selected === item.id} onclick={() => (selected = item.id)}>
          <strong>{item.label}</strong>
          <div class="muted small">{item.hint}</div>
        </button>
      {/each}
    </div>
    <p class="muted small" style="margin-top: 0.8rem">
      Source fetch remains shell-only until a separate network-boundary decision.
    </p>
  </div>

  <div class="card">
    <h2>{stage.label}</h2>
    <div class="fields">
      {#each stage.fields as field (field)}
        <label>
          <span>{field}</span>
          <input bind:value={fields[field]} placeholder={field === "out" ? "optional output root" : field} />
        </label>
      {/each}
    </div>
    <button class="primary" onclick={run} disabled={busy}>
      {busy ? "Running…" : "Run stage"}
    </button>
    {#if error}
      <p class="error">{error}</p>
    {:else if result}
      <pre>{JSON.stringify(result, null, 2)}</pre>
    {:else}
      <p class="muted">Run the stage to see its artifact paths and summary.</p>
    {/if}
  </div>
</div>

<style>
  .fields {
    display: grid;
    gap: 0.6rem;
    margin-bottom: 0.8rem;
  }

  label {
    display: grid;
    gap: 0.25rem;
  }

  label span {
    color: var(--muted);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  input {
    padding: 0.55rem;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
    color: inherit;
  }

  pre {
    margin-top: 0.8rem;
    max-height: 22rem;
    overflow: auto;
    padding: 0.8rem;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
    font-size: 0.8rem;
  }
</style>
