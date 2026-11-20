/** Runtime history over the shared client API. */

import { call, callVoid } from "./api";
import type { DatasetSummary, RunSummary, StageRunSummary } from "./types";
import type { ImprovementRunSummary } from "./research";
import type { SimulationSummary } from "./simulation";

/** The runtime kinds the History screen lists. */
export type HistoryKind = "run" | "stage" | "research" | "simulation" | "dataset";

/** One normalized history row. */
export interface HistoryEntry {
  kind: HistoryKind;
  id: string;
  state: string;
  timestamp: string;
  archivedAt: string | null;
  bytes?: number;
}

/** The label shown on a tab. */
export const HISTORY_KINDS: { id: HistoryKind; label: string }[] = [
  { id: "run", label: "Runs" },
  { id: "stage", label: "Stages" },
  { id: "research", label: "Research" },
  { id: "simulation", label: "Simulations" },
  { id: "dataset", label: "Datasets" },
];

/** Whether a kind can be archived. */
export function canArchive(kind: HistoryKind): boolean {
  return kind !== "stage";
}

/** Read one kind. */
export async function list(
  kind: HistoryKind,
  includeArchived: boolean,
): Promise<HistoryEntry[]> {
  switch (kind) {
    case "run": {
      const rows = await call<RunSummary[]>(
        `/runs?limit=200${includeArchived ? "&include_archived=true" : ""}`,
      );
      return rows.map((row) => ({
        kind,
        id: row.run_id,
        state: row.status,
        timestamp: row.submitted_at,
        archivedAt: row.archived_at ?? null,
      }));
    }
    case "stage": {
      const rows = await call<StageRunSummary[]>("/stages/runs?limit=200");
      return rows.map((row) => ({
        kind,
        id: row.stage_run_id,
        state: row.state,
        timestamp: row.submitted_at,
        archivedAt: null,
      }));
    }
    case "research": {
      const rows = await call<ImprovementRunSummary[]>(
        `/research/runs?limit=200${includeArchived ? "&include_archived=true" : ""}`,
      );
      return rows.map((row) => ({
        kind,
        id: row.improvement_id,
        state: row.state,
        timestamp: row.created_at,
        archivedAt: row.archived_at ?? null,
      }));
    }
    case "simulation": {
      const rows = await call<SimulationSummary[]>(
        `/simulation/runs?limit=200${includeArchived ? "&include_archived=true" : ""}`,
      );
      return rows.map((row) => ({
        kind,
        id: row.sim_id,
        state: row.status,
        timestamp: "",
        archivedAt: row.archived_at ?? null,
      }));
    }
    case "dataset": {
      const rows = await call<DatasetSummary[]>(
        `/catalog/datasets${includeArchived ? "?include_archived=true" : ""}`,
      );
      return rows.map((row) => ({
        kind,
        id: row.data_version,
        state: row.name,
        timestamp: "",
        archivedAt: row.archived_at ?? null,
        bytes: row.bytes,
      }));
    }
  }
}

/** Archive one record. */
export async function archive(kind: HistoryKind, id: string): Promise<void> {
  const path =
    kind === "run"
      ? `/runs/${encodeURIComponent(id)}/archive`
      : kind === "research"
        ? `/research/runs/${encodeURIComponent(id)}/archive`
        : kind === "simulation"
          ? `/simulation/runs/${encodeURIComponent(id)}/archive`
          : `/catalog/datasets/${encodeURIComponent(id)}/archive`;
  await call(path, { method: "POST" });
}

/** Erase one archived record. */
export async function erase(kind: HistoryKind, id: string, confirm: string): Promise<void> {
  if (confirm !== id) {
    throw new Error("the confirmation must equal the record id");
  }
  if (kind === "stage") {
    await callVoid(`/stages/runs/${encodeURIComponent(id)}`, { method: "DELETE" });
    return;
  }
  const path =
    kind === "run"
        ? `/runs/${encodeURIComponent(id)}?confirm=${encodeURIComponent(confirm)}`
        : kind === "research"
          ? `/research/runs/${encodeURIComponent(id)}?confirm=${encodeURIComponent(confirm)}`
          : kind === "simulation"
            ? `/simulation/runs/${encodeURIComponent(id)}?confirm=${encodeURIComponent(confirm)}`
            : `/catalog/datasets/${encodeURIComponent(id)}?confirm=${encodeURIComponent(confirm)}`;
  await call(path, { method: "DELETE" });
}
