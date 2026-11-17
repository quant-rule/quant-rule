/**
 * Diagnostic stage reads and their verified artifact download.
 *
 * The browser only reads published stage artifacts. It filters `events.csv`
 * rows by the requested timeframe and reads coverage fields from `report.json`;
 * it never computes an indicator, event, or coverage value. Downloads are
 * checked against the stage record's size and SHA-256 before they are offered.
 */

import { fetchBytes } from "./api";
import { sha256Hex } from "./artifacts";
import type {
  ArtifactRef,
  Digest,
  StageCoverage,
  StageEventRow,
  StageEventView,
  StageRunDetail,
  StageRunSummary,
} from "./types";

/** One model stage response fixture, as the shared parity view expects it. */
export interface StageFetchView {
  stage_run_id: Digest;
  artifacts: { name: string; bytes: number; sha256: Digest }[];
}

/** Encode one stage id for a path segment. */
export function stagePath(stageRunId: Digest): string {
  return `/stages/runs/${encodeURIComponent(stageRunId)}`;
}

/** Encode one artifact name as relative path segments. */
export function stageArtifactPath(stageRunId: Digest, name: string): string {
  const path = name
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");
  return `${stagePath(stageRunId)}/artifacts/${path}`;
}

/** Convert a timeframe short code to seconds. */
export function timeframeSeconds(value: string): number {
  const trimmed = value.trim();
  const match = /^(\d+)([smhd])?$/.exec(trimmed);
  const count = match?.[1];
  if (count === undefined) {
    throw new Error(`timeframe '${value}' is not a ladder code such as 5m or 1h`);
  }
  const amount = Number(count);
  const unit = match?.[2] ?? "s";
  const seconds =
    unit === "m"
      ? amount * 60
      : unit === "h"
        ? amount * 3_600
        : unit === "d"
          ? amount * 86_400
          : amount;
  if (seconds === 0) {
    throw new Error("timeframe must be greater than zero");
  }
  return seconds;
}

/** Split one comma-separated artifact line. */
function fields(line: string): string[] {
  return line.replace(/\r$/, "").split(",");
}

/**
 * Filter `events.csv` to one timeframe.
 *
 * The header and row order are the artifact's; the function adds nothing.
 */
export function filterEvents(
  csv: string,
  timeframe: string,
): { columns: string[]; events: StageEventRow[] } {
  const seconds = timeframeSeconds(timeframe);
  const lines = csv.split("\n").filter((line) => line.length > 0);
  const header = lines.shift();
  if (header === undefined) {
    throw new Error("events.csv carries no header");
  }
  const columns = fields(header);
  const timeframeColumn = columns.indexOf("timeframe_secs");
  if (timeframeColumn < 0) {
    throw new Error("events.csv has no timeframe_secs column");
  }
  const events: StageEventRow[] = [];
  for (const line of lines) {
    const values = fields(line);
    if (values.length !== columns.length) {
      throw new Error(`events.csv row has ${values.length} fields, not ${columns.length}`);
    }
    if (values[timeframeColumn] !== String(seconds)) {
      continue;
    }
    const row: StageEventRow = {};
    columns.forEach((column, index) => {
      row[column] = values[index] ?? "";
    });
    events.push(row);
  }
  return { columns, events };
}

/** Read the four coverage fields from `report.json`. */
export function reportCoverage(report: string): StageCoverage[] {
  const value = JSON.parse(report) as { timeframes?: unknown };
  if (!Array.isArray(value.timeframes)) {
    throw new Error("report.json carries no timeframes array");
  }
  const coverage = value.timeframes.map((entry) => {
    if (entry === null || typeof entry !== "object") {
      throw new Error("a report timeframe is not an object");
    }
    const row = entry as Record<string, unknown>;
    if (typeof row.timeframe_secs !== "number") {
      throw new Error("a report timeframe has no timeframe_secs");
    }
    return {
      timeframe_secs: row.timeframe_secs,
      bars: typeof row.bars === "number" ? row.bars : null,
      expected_bars: typeof row.expected_bars === "number" ? row.expected_bars : null,
      coverage_share:
        typeof row.coverage_share === "number" ? row.coverage_share : null,
    };
  });
  coverage.sort((left, right) => left.timeframe_secs - right.timeframe_secs);
  return coverage;
}

/** Build the normalized event view from the two published artifacts. */
export function stageEventView(
  stageRunId: Digest,
  timeframe: string,
  eventsCsv: string,
  reportJson: string | null,
): StageEventView {
  const { columns, events } = filterEvents(eventsCsv, timeframe);
  return {
    stage_run_id: stageRunId,
    timeframe,
    columns,
    events,
    coverage: reportJson === null ? [] : reportCoverage(reportJson),
  };
}

/** Fetch one stage artifact without interpreting it. */
export async function stageArtifactBytes(
  stageRunId: Digest,
  name: string,
): Promise<ArrayBuffer> {
  return fetchBytes(stageArtifactPath(stageRunId, name));
}

/** Fetch one artifact and verify it against the stage record. */
export async function downloadStageArtifact(
  stageRunId: Digest,
  artifact: ArtifactRef,
): Promise<ArrayBuffer> {
  const bytes = await stageArtifactBytes(stageRunId, artifact.name);
  const digest = await sha256Hex(bytes);
  if (digest !== artifact.sha256 || bytes.byteLength !== artifact.bytes) {
    throw new Error(
      `artifact ${artifact.name} does not match the stage record; the download was refused`,
    );
  }
  return bytes;
}

/** Fetch every artifact of a stage and return its verified normalized view. */
export async function fetchStageArtifacts(
  detail: StageRunDetail,
): Promise<StageFetchView> {
  if (detail.artifact === null) {
    throw new Error(`stage ${detail.stage_run_id} has not produced an artifact`);
  }
  const artifacts = [...detail.artifact.files].sort((left, right) =>
    left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
  );
  const verified: StageFetchView["artifacts"] = [];
  for (const artifact of artifacts) {
    const bytes = await downloadStageArtifact(detail.stage_run_id, artifact);
    verified.push({
      name: artifact.name,
      bytes: bytes.byteLength,
      sha256: await sha256Hex(bytes),
    });
  }
  return { stage_run_id: detail.stage_run_id, artifacts: verified };
}

/** A compact stage or data id for a table cell. */
export function stageShortId(value: string): string {
  return value.length > 18 ? `${value.slice(0, 15)}…` : value;
}

/** A stage summary row's artifact id, or an em dash. */
export function artifactId(stage: StageRunSummary): string {
  return stage.artifact_id ?? "—";
}
