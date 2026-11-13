import { describe, expect, it } from "bun:test";
import { bytes, elapsed, isActive, pollDelay, shortId, submitted } from "./runs";
import type { RunSummary } from "./types";

function run(status: RunSummary["status"]): RunSummary {
  return {
    run_id: "sha256:" + "a".repeat(64),
    status,
    config_id: "cfg_abc",
    data_version: "sha256:" + "b".repeat(64),
    submitted_at: "2026-11-13T12:00:00Z",
    elapsed_ms: null,
  };
}

describe("isActive", () => {
  it("is true only for queued and running", () => {
    expect(isActive("queued")).toBe(true);
    expect(isActive("running")).toBe(true);
    expect(isActive("succeeded")).toBe(false);
    expect(isActive("failed")).toBe(false);
    expect(isActive("cancelled")).toBe(false);
  });
});

describe("pollDelay", () => {
  it("polls fast while a run is active", () => {
    expect(pollDelay([run("succeeded"), run("running")])).toBe(2_000);
  });

  it("polls slowly when every run is terminal", () => {
    expect(pollDelay([run("succeeded"), run("failed")])).toBe(30_000);
    expect(pollDelay([])).toBe(30_000);
  });
});

describe("shortId", () => {
  it("keeps a short value and truncates a digest", () => {
    expect(shortId("cfg_abc")).toBe("cfg_abc");
    expect(shortId("sha256:" + "a".repeat(64))).toBe("sha256:aaaaaaaa…");
  });
});

describe("elapsed", () => {
  it("renders a duration or an em dash", () => {
    expect(elapsed(null)).toBe("—");
    expect(elapsed(1_500)).toBe("1.5s");
    expect(elapsed(12_345)).toBe("12s");
  });
});

describe("submitted", () => {
  it("renders an RFC 3339 instant as a UTC table value", () => {
    expect(submitted("2026-11-13T12:00:00Z")).toBe("2026-11-13 12:00:00Z");
  });
});

describe("bytes", () => {
  it("renders a fixed byte count", () => {
    expect(bytes(512)).toBe("512 B");
    expect(bytes(2_048)).toBe("2.0 KiB");
    expect(bytes(2 * 1_024 * 1_024)).toBe("2.0 MiB");
  });
});
