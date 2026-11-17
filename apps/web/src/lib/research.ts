/**
 * The research-ledger transport seam.
 *
 * The browser reads the v3 pre-registration, submits the coupled run, compares
 * published numbers, records the human decision, and reads the append-only
 * registries. It never computes an id, a comparison, or a decision.
 */

import { call } from "./api";

export type ImprovementTier = "model" | "strategy" | "portfolio";
export type ImprovementPartition =
  | "model_dev"
  | "model_validation"
  | "strategy_dev"
  | "strategy_validation"
  | "portfolio_dev"
  | "portfolio_holdout";
export type ImprovementState = "created" | "submitted" | "decided";
export type ImprovementDecision =
  | "accepted"
  | "rejected"
  | "no-edge"
  | "invalidated"
  | "new-hypothesis";

export interface ImprovementManifest {
  schema: string;
  improvement_id: string;
  tier: ImprovementTier;
  hypothesis: string;
  created_utc: string;
  changed: ImprovementTier;
  frozen: ImprovementTier[];
  partition: ImprovementPartition;
  data_version: string;
  config_id: string;
  criteria_id: string | null;
  green_criteria: Record<string, unknown>;
  holdout_access: "none" | "final";
  baseline: string | null;
  code_commit: string;
  seed: number;
  command: string | null;
  analytics: boolean | null;
}

export interface ImprovementRunSummary {
  improvement_id: string;
  tier: ImprovementTier;
  state: ImprovementState;
  decision: ImprovementDecision | null;
  created_at: string;
  submitted_at: string | null;
  decided_at: string | null;
}

export interface ImprovementRunDetail extends ImprovementRunSummary {
  decision_note: string | null;
  manifest: ImprovementManifest;
  pipeline_run_id: string | null;
  canonical_digest: string | null;
}

export interface BaselineRecord {
  baseline_id: string;
  tier: ImprovementTier;
  run_id: string | null;
  improvement_id: string | null;
  accepted_by: string;
  accepted_on: string;
  scope: string;
  supersedes: string | null;
  created_at: string;
}

export interface InvalidationRecord {
  invalidation_id: string;
  date: string;
  commit: string;
  removed: string;
  reason: string;
  superseded_by: string | null;
  created_at: string;
}

export interface ComparisonRow {
  metric: string;
  candidate: number | null;
  baseline: number | null;
  delta: number | null;
}

export interface ImprovementComparison {
  improvement_id: string;
  tier: ImprovementTier;
  candidate_run_id: string;
  baseline_id: string | null;
  candidate_digest: string | null;
  baseline_digest: string | null;
  rows: ComparisonRow[];
  note: string | null;
}

export interface CreateImprovement {
  tier: ImprovementTier;
  hypothesis: string;
  partition: ImprovementPartition;
  data_version: string;
  config_id: string;
  criteria_id: string | null;
  green_criteria: Record<string, unknown>;
  baseline: string | null;
  code_commit: string;
  seed: number;
  analytics: boolean | null;
}

/** Whether a create form carries every field the endpoint needs. */
export function canCreate(input: CreateImprovement): boolean {
  const criteria =
    input.tier === "model" ||
    (input.criteria_id !== null && input.criteria_id.trim().length > 0);
  return (
    criteria &&
    input.hypothesis.trim().length > 0 &&
    input.data_version.trim().length > 0 &&
    input.config_id.trim().length > 0 &&
    input.code_commit.trim().length > 0 &&
    Number.isInteger(input.seed) &&
    input.seed >= 0 &&
    Object.keys(input.green_criteria).length > 0
  );
}

/** The list path, with its optional filters. */
export function runsPath(
  options: {
    tier?: ImprovementTier;
    state?: ImprovementState;
    limit?: number;
  } = {},
): string {
  const query = new URLSearchParams();
  if (options.tier !== undefined) {
    query.set("tier", options.tier);
  }
  if (options.state !== undefined) {
    query.set("state", options.state);
  }
  if (options.limit !== undefined) {
    query.set("limit", String(options.limit));
  }
  const suffix = query.toString();
  return suffix === "" ? "/research/runs" : `/research/runs?${suffix}`;
}

/** The detail path for one improvement run. */
export function runPath(improvementId: string): string {
  return `/research/runs/${encodeURIComponent(improvementId)}`;
}

/** The submit path for one improvement run. */
export function submitPath(improvementId: string): string {
  return `${runPath(improvementId)}/submit`;
}

/** The comparison path, with an optional baseline override. */
export function comparePath(improvementId: string, baseline?: string): string {
  const base = `${runPath(improvementId)}/compare`;
  return baseline === undefined || baseline.trim() === ""
    ? base
    : `${base}?baseline=${encodeURIComponent(baseline)}`;
}

/** The decision path for one improvement run. */
export function decisionPath(improvementId: string): string {
  return `${runPath(improvementId)}/decision`;
}

/** The baseline list path. */
export function baselinesPath(): string {
  return "/research/baselines";
}

/** The baseline-by-tier path. */
export function baselinePath(tier: ImprovementTier): string {
  return `/research/baselines/${tier}`;
}

/** The invalidation list path. */
export function invalidationsPath(): string {
  return "/research/invalidations";
}

/** Read the improvement-run list. */
export async function listRuns(
  options: {
    tier?: ImprovementTier;
    state?: ImprovementState;
    limit?: number;
  } = {},
): Promise<ImprovementRunSummary[]> {
  return call<ImprovementRunSummary[]>(runsPath(options));
}

/** Create one pre-registration and return the server-assigned record. */
export async function createRun(
  input: CreateImprovement,
): Promise<ImprovementRunDetail> {
  return call<ImprovementRunDetail>("/research/runs", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

/** Read one improvement run. */
export async function fetchRun(
  improvementId: string,
): Promise<ImprovementRunDetail> {
  return call<ImprovementRunDetail>(runPath(improvementId));
}

/** Submit the coupled pipeline run. */
export async function submitRun(
  improvementId: string,
): Promise<ImprovementRunDetail> {
  return call<ImprovementRunDetail>(submitPath(improvementId), {
    method: "POST",
  });
}

/** Read the published-number comparison. */
export async function compareRun(
  improvementId: string,
  baseline?: string,
): Promise<ImprovementComparison> {
  return call<ImprovementComparison>(comparePath(improvementId, baseline));
}

/** Record one human decision. */
export async function decideRun(
  improvementId: string,
  decision: ImprovementDecision,
  note: string,
): Promise<ImprovementRunDetail> {
  return call<ImprovementRunDetail>(decisionPath(improvementId), {
    method: "POST",
    body: JSON.stringify({ decision, note }),
  });
}

/** Read the accepted baselines. */
export async function listBaselines(): Promise<BaselineRecord[]> {
  return call<BaselineRecord[]>(baselinesPath());
}

/** Read the current baseline for one tier. */
export async function fetchBaseline(
  tier: ImprovementTier,
): Promise<BaselineRecord> {
  return call<BaselineRecord>(baselinePath(tier));
}

/** Read the invalidation tombstones. */
export async function listInvalidations(): Promise<InvalidationRecord[]> {
  return call<InvalidationRecord[]>(invalidationsPath());
}
