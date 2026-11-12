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
    default:
      throw new Error(`unknown mock command: ${command}`);
  }
}
