import type {
  ClaimView,
  DatasetSummaryView,
  MemoryStatusView,
  MetricView,
  ModelRunView,
  NoteView,
  RecordedRunView,
  RunRecordView,
  RunSummaryView,
  WorkspaceRecordView,
  WorkspaceView,
} from "./types";

const runSummary: RunSummaryView = {
  name: "btcusd-2024",
  output_root: "/mock/workspace/target/report",
  config_hash: "0123456789abcdef",
  model_id: "M1.0",
  execution_id: "E1.0",
  portfolio_id: "P1.0",
  instrument: "BTCUSD",
  data_version: "mock-btcusd-2024",
  data_source: "/mock/workspace/data/sources/BTCUSD_M1_2024.csv",
  data_timeframes: ["1m", "5m", "15m", "1h"],
  model_timeframes: ["1m", "5m", "15m", "1h"],
  execution_fill_timeframe: "5m",
  initial_cash: 10000,
  leverage: 1,
  max_open_positions: 1,
};

const workspace: WorkspaceView = {
  root: "/mock/workspace",
  name: "workspace",
  datasets: [
    { name: "BTCUSD_M1_2024", path: "/mock/workspace/data/sources/BTCUSD_M1_2024.csv" },
  ],
  runs: ["/mock/workspace/config/examples/btcusd-2024/run.json"],
};

const workspaces: WorkspaceRecordView[] = [];
const runs: RunRecordView[] = [];

function requireString(args: Record<string, unknown>, key: string): string {
  const value = args[key];
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${key} is required`);
  }
  return value;
}

/**
 * Deterministic fixture backend for the browser and Playwright.
 *
 * It mirrors the Rust view shapes; it never reads the filesystem and never
 * performs a computation. The native build does not use it.
 */
export async function mockInvoke<T>(
  command: string,
  args: Record<string, unknown>,
): Promise<T> {
  switch (command) {
    case "open_workspace": {
      requireString(args, "root");
      return workspace as T;
    }
    case "dataset_summary": {
      requireString(args, "path");
      const summary: DatasetSummaryView = {
        name: "BTCUSD_M1_2024",
        path: String(args.path),
        candles: 527040,
        first_open_time: 1704067200,
        last_open_time: 1735689599,
      };
      return summary as T;
    }
    case "check_run": {
      requireString(args, "config");
      return runSummary as T;
    }
    case "run_model": {
      requireString(args, "config");
      const model: ModelRunView = {
        run: runSummary,
        timeframes: [
          { timeframe_secs: 60, bars: 527040, events: 18342, levels: 412 },
          { timeframe_secs: 300, bars: 105408, events: 6210, levels: 188 },
          { timeframe_secs: 900, bars: 35136, events: 2401, levels: 74 },
          { timeframe_secs: 3600, bars: 8784, events: 702, levels: 21 },
        ],
        events: 27655,
      };
      return model as T;
    }
    case "export_run_summary": {
      requireString(args, "config");
      requireString(args, "out");
      return runSummary as T;
    }
    case "memory_status": {
      const status: MemoryStatusView = {
        schema: "quantrule.research-memory/v1",
        version: 1,
        path: String(args.db ?? ""),
      };
      return status as T;
    }
    case "memory_workspaces":
      return workspaces as T;
    case "memory_runs":
      return runs as T;
    case "memory_metrics": {
      const metrics: MetricView[] = [
        { id: 1, run_id: Number(args.run_id), name: "profit_factor", kind: "real", value: 1.42 },
        { id: 2, run_id: Number(args.run_id), name: "trades", kind: "integer", value: 318 },
        { id: 3, run_id: Number(args.run_id), name: "sharpe", kind: "null", value: null },
      ];
      return metrics as T;
    }
    case "memory_claims": {
      const claims: ClaimView[] = [
        {
          id: 1,
          run_id: Number(args.run_id),
          claim_type: "Behaviour Claim",
          statement: "the model run is deterministic",
          status: "accepted",
          created_at: 1763000000,
        },
      ];
      return claims as T;
    }
    case "memory_notes": {
      const notes: NoteView[] = [
        {
          id: 1,
          workspace_id: 1,
          run_id: Number(args.run_id ?? 1),
          body: "fixture note",
          created_at: 1763000000,
          updated_at: 1763000000,
        },
      ];
      return notes as T;
    }
    case "memory_notes_for_run": {
      requireString(args, "db");
      const notes: NoteView[] = [
        {
          id: 1,
          workspace_id: 1,
          run_id: Number(args.run_id ?? 1),
          body: "fixture note",
          created_at: 1763000000,
          updated_at: 1763000000,
        },
      ];
      return notes as T;
    }
    case "memory_metric_set": {
      requireString(args, "db");
      requireString(args, "name");
      requireString(args, "kind");
      return 1 as T;
    }
    case "memory_metric_delete": {
      requireString(args, "db");
      return true as T;
    }
    case "memory_claim_add": {
      requireString(args, "db");
      requireString(args, "statement");
      return 1 as T;
    }
    case "memory_claim_delete": {
      requireString(args, "db");
      return true as T;
    }
    case "memory_note_add": {
      requireString(args, "db");
      requireString(args, "body");
      return 1 as T;
    }
    case "memory_note_update": {
      requireString(args, "db");
      requireString(args, "body");
      return true as T;
    }
    case "memory_note_delete": {
      requireString(args, "db");
      return true as T;
    }
    case "record_run": {
      requireString(args, "config");
      requireString(args, "run_key");
      const recorded: RecordedRunView = {
        workspace_id: 1,
        dataset_id: 1,
        config_id: 1,
        run_id: runs.length + 1,
      };
      if (workspaces.length === 0) {
        workspaces.push({ id: 1, root: workspace.root, name: workspace.name, created_at: 1763000000 });
      }
      runs.push({
        id: recorded.run_id,
        workspace_id: 1,
        config_id: 1,
        run_key: String(args.run_key),
        data_version: runSummary.data_version,
        config_hash: runSummary.config_hash,
        status: String(args.status ?? "complete"),
        started_at: Number(args.started_at ?? 1763000000),
        finished_at: (args.finished_at as number | null) ?? null,
        summary_json: JSON.stringify(runSummary),
      });
      return recorded as T;
    }
    case "memory_export":
      requireString(args, "db");
      requireString(args, "out");
      return { path: String(args.out) } as T;
    case "memory_import":
      requireString(args, "db");
      requireString(args, "from");
      return { imported: true } as T;
    case "memory_backup":
      requireString(args, "db");
      requireString(args, "out");
      return { path: String(args.out) } as T;
    case "memory_restore":
      requireString(args, "backup");
      requireString(args, "target");
      return { path: String(args.target) } as T;
    case "inspect_candles": {
      requireString(args, "path");
      return { candles: 527040 } as T;
    }
    case "inspect_candles_tolerant": {
      requireString(args, "path");
      return { candles: 527040 } as T;
    }
    case "inspect_timeframes": {
      requireString(args, "path");
      return {
        timeframes: [
          { timeframe: "1m", candles: 527040 },
          { timeframe: "5m", candles: 105408 },
          { timeframe: "15m", candles: 35136 },
          { timeframe: "1h", candles: 8784 },
        ],
      } as T;
    }
    case "compute_events": {
      requireString(args, "path");
      return { events: 27655, timeframes: [{ timeframe: "300", candles: 105408 }] } as T;
    }
    case "run_model_report": {
      requireString(args, "config");
      return { root: String(args.out ?? "/mock/workspace/target/model"), artifacts: ["readings.csv", "events.csv", "levels.csv", "legs.csv", "summary.txt", "report.json", "result.json", "provenance.json"] } as T;
    }
    case "explain_strategy": {
      requireString(args, "model");
      requireString(args, "strategy");
      return { decision: "hold", position_before: "flat", position_after: "flat" } as T;
    }
    case "run_strategy": {
      requireString(args, "model");
      requireString(args, "strategy");
      return { root: "/mock/workspace/target/strategy", artifacts: ["decisions.ndjson", "summary.txt", "provenance.json"] } as T;
    }
    case "run_execution": {
      requireString(args, "decisions");
      requireString(args, "config");
      return { root: "/mock/workspace/target/execution", artifacts: ["orders.csv", "fills.csv", "rejections.csv", "summary.txt", "provenance.json"] } as T;
    }
    case "replay_portfolio": {
      requireString(args, "fills");
      requireString(args, "config");
      return { root: "/mock/workspace/target/portfolio", artifacts: ["trades.csv", "equity.csv", "funding.csv", "risk_events.csv", "metrics.json", "summary.txt", "provenance.json"] } as T;
    }
    case "run_backtest": {
      requireString(args, "config");
      requireString(args, "strategy");
      return { root: "/mock/workspace/target/backtest", manifest: "/mock/workspace/target/backtest/manifest.json", stages: ["model", "strategy", "execution", "portfolio"] } as T;
    }
    case "qualify": {
      requireString(args, "evidence");
      requireString(args, "criteria");
      return { root: "/mock/workspace/target/qualification", qualification: "/mock/workspace/target/qualification/qualification.json", verdict: "insufficient_evidence", summary: "total=1 pass=0 fail=0 insufficient_evidence=1" } as T;
    }
    case "evaluate_analytics": {
      requireString(args, "evidence");
      return { root: "/mock/workspace/target/analytics", metrics: "/mock/workspace/target/analytics/metrics.json", tables: "/mock/workspace/target/analytics/tables", summary: "/mock/workspace/target/analytics/summary.txt" } as T;
    }
    default:
      throw new Error(`unknown mock command: ${command}`);
  }
}
