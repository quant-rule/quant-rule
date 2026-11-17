/**
 * The Operations screen's reads and deterministic text.
 *
 * Retention status is operational metadata: it describes the evidence
 * service's startup cleanup, not a model measurement or an evidence record.
 * Every formatter is pure so the same fixture renders the same text.
 */
import { call } from "./api";
import type { CleanupReport, OperationsReport } from "./types";

/** Read the evidence service's retention and cleanup snapshot. */
export function loadOperations(): Promise<OperationsReport> {
  return call<OperationsReport>("/operations");
}

/** The retention window as text; one day is not "1 days". */
export function retentionWindow(days: number): string {
  return `${days} ${days === 1 ? "day" : "days"}`;
}

/** When the last startup cleanup ran, or that none has run. */
export function cleanupTime(report: OperationsReport): string {
  return report.last_cleanup?.completed_utc ?? "No startup cleanup recorded";
}

/** What the last cleanup removed, or the error that stopped it. */
export function cleanupCounts(cleanup: CleanupReport | null): string {
  if (cleanup === null) {
    return "Nothing removed yet";
  }
  if (cleanup.error !== null) {
    return `Failed: ${cleanup.error}`;
  }
  return `${cleanup.stages_removed} stage runs and ${cleanup.objects_removed} objects removed`;
}
