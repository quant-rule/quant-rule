import { describe, expect, it } from "bun:test";
import {
  cleanupCounts,
  cleanupTime,
  retentionWindow,
} from "./operations";
import type { OperationsReport } from "./types";

function report(overrides: Partial<OperationsReport> = {}): OperationsReport {
  return {
    stage_retention_days: 30,
    stage_store: true,
    last_cleanup: {
      completed_utc: "2026-11-17T08:00:00Z",
      stages_removed: 3,
      objects_removed: 4,
      error: null,
    },
    ...overrides,
  };
}

describe("retentionWindow", () => {
  it("keeps the singular and plural forms", () => {
    expect(retentionWindow(1)).toBe("1 day");
    expect(retentionWindow(30)).toBe("30 days");
  });
});

describe("cleanupTime and cleanupCounts", () => {
  it("reports the last cleanup and its counts", () => {
    expect(cleanupTime(report())).toBe("2026-11-17T08:00:00Z");
    expect(cleanupCounts(report().last_cleanup)).toBe(
      "3 stage runs and 4 objects removed",
    );
  });

  it("reports no cleanup distinctly from a zero-count cleanup", () => {
    const empty = report({ last_cleanup: null });
    expect(cleanupTime(empty)).toBe("No startup cleanup recorded");
    expect(cleanupCounts(null)).toBe("Nothing removed yet");
    expect(
      cleanupCounts({
        completed_utc: "2026-11-17T08:00:00Z",
        stages_removed: 0,
        objects_removed: 0,
        error: null,
      }),
    ).toBe("0 stage runs and 0 objects removed");
  });

  it("keeps a failure and its message visible", () => {
    expect(
      cleanupCounts({
        completed_utc: "2026-11-17T08:00:00Z",
        stages_removed: 0,
        objects_removed: 0,
        error: "the object store refused a delete",
      }),
    ).toBe("Failed: the object store refused a delete");
  });
});
