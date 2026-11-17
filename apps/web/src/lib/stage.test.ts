import { describe, expect, it } from "bun:test";
import {
  filterEvents,
  reportCoverage,
  stageArtifactPath,
  stageEventView,
  stagePath,
  timeframeSeconds,
} from "./stage";

const EVENTS =
  "timeframe_secs,event_index,event_code\n300,0,1\n900,0,2\n300,1,3\n";
const REPORT =
  '{"timeframes":[{"timeframe_secs":900,"bars":0,"expected_bars":1,"coverage_share":0.0},' +
  '{"timeframe_secs":300,"bars":3,"expected_bars":3,"coverage_share":1.0}]}';

describe("stagePath", () => {
  it("encodes the stage id as one segment", () => {
    expect(stagePath("sha256:abc")).toBe("/stages/runs/sha256%3Aabc");
    expect(stageArtifactPath("sha256:abc", "tables/equity.csv")).toBe(
      "/stages/runs/sha256%3Aabc/artifacts/tables/equity.csv",
    );
  });
});

describe("timeframeSeconds", () => {
  it("maps ladder codes and bare seconds", () => {
    expect(timeframeSeconds("1m")).toBe(60);
    expect(timeframeSeconds("5m")).toBe(300);
    expect(timeframeSeconds("1h")).toBe(3_600);
    expect(timeframeSeconds("300")).toBe(300);
  });

  it("refuses an empty or zero timeframe", () => {
    expect(() => timeframeSeconds("")).toThrow();
    expect(() => timeframeSeconds("0m")).toThrow();
    expect(() => timeframeSeconds("week")).toThrow();
  });
});

describe("filterEvents", () => {
  it("keeps only the requested timeframe in artifact order", () => {
    const filtered = filterEvents(EVENTS, "5m");
    expect(filtered.columns).toEqual([
      "timeframe_secs",
      "event_index",
      "event_code",
    ]);
    expect(filtered.events.map((row) => row.event_code)).toEqual(["1", "3"]);
    expect(filtered.events.every((row) => row.timeframe_secs === "300")).toBe(true);
  });

  it("refuses a malformed row", () => {
    expect(() => filterEvents("timeframe_secs,event_index\n300\n", "5m")).toThrow();
    expect(() => filterEvents("", "5m")).toThrow();
  });
});

describe("reportCoverage", () => {
  it("keeps the four coverage fields and sorts by timeframe", () => {
    const coverage = reportCoverage(REPORT);
    expect(coverage).toEqual([
      { timeframe_secs: 300, bars: 3, expected_bars: 3, coverage_share: 1.0 },
      { timeframe_secs: 900, bars: 0, expected_bars: 1, coverage_share: 0.0 },
    ]);
  });

  it("refuses a report without a timeframes array", () => {
    expect(() => reportCoverage("{}")).toThrow();
  });
});

describe("stageEventView", () => {
  it("builds one deterministic normalized view", () => {
    const view = stageEventView("sha256:stage", "5m", EVENTS, REPORT);
    expect(view.stage_run_id).toBe("sha256:stage");
    expect(view.events).toHaveLength(2);
    expect(view.coverage).toHaveLength(2);
  });

  it("omits coverage when report.json is absent", () => {
    const view = stageEventView("sha256:stage", "5m", EVENTS, null);
    expect(view.coverage).toEqual([]);
  });
});
