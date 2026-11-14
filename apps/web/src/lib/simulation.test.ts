import { afterEach, describe, expect, it } from "bun:test";
import { ApiError } from "./api";
import {
  canCreate,
  checkPath,
  createRun,
  decisionPath,
  deleteMark,
  fetchMarks,
  fetchRun,
  fetchSeries,
  listRuns,
  marksPath,
  oraclePath,
  postCheck,
  postDecision,
  postMark,
  postOracle,
  runPath,
  runsPath,
  selectWindows,
  seriesPath,
  windowsPath,
  type SimulationCheck,
  type SimulationCreate,
  type SimulationRun,
} from "./simulation";
import type { ChartSeries, MarkDraft } from "./chart/types";

const original = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = original;
});

interface Seen {
  url: string;
  init: RequestInit;
}

function capture(status: number, body: unknown): Seen[] {
  const seen: Seen[] = [];
  globalThis.fetch = (async (input: Parameters<typeof fetch>[0], init?: RequestInit) => {
    seen.push({ url: String(input), init: init ?? {} });
    const responseBody = body === undefined ? "" : JSON.stringify(body);
    return new Response(responseBody, {
      status,
      headers: { "content-type": "application/json" },
    });
  }) as unknown as typeof fetch;
  return seen;
}

const DRAFT: MarkDraft = {
  chart: 1,
  bar: 17,
  action: "entry",
  quality: "good",
  side: "long",
  note: "higher low",
};

const CREATE: SimulationCreate = {
  premise: "higher lows into a confirmed level",
  bundle: "sha256:" + "a".repeat(64),
  inputs: "EMA stack and RSI zone",
  seed: 7,
  criteria: "min_events=30; min_mean_net>0; min_hit_rate>0.5",
};

const RAW_MARK = {
  mark_id: "m1",
  chart: "1",
  bar: 4,
  marked_time_utc: "2026-11-14T10:00:00Z",
  action: "entry" as const,
  quality: "good" as const,
  side: "long" as const,
  origin: "human" as const,
  note: "higher low",
};

const RUN: SimulationRun = {
  sim_id: "sim",
  premise: "higher lows into a confirmed level",
  status: "marking",
  bundle: CREATE.bundle,
  inputs: CREATE.inputs,
  inputs_digest: "sha256:" + "b".repeat(64),
  seed: 7,
  criteria: CREATE.criteria,
  counts: { windows: 2, human_marks: 1, oracle_marks: 0 },
  decision: null,
};

const CHECK: SimulationCheck = {
  events: 42,
  mean_net: 0.12,
  hit_rate: 0.57,
  control: 80,
  control_mean_net: 0.05,
  control_hit_rate: 0.51,
  positive_half_years: 4,
  passed: true,
  failures: [],
};

describe("simulation paths", () => {
  it("encodes the simulation id and chart number", () => {
    expect(seriesPath("sim/one", 2)).toBe(
      "/simulation/runs/sim%2Fone/windows/2/series",
    );
    expect(marksPath("sim/one")).toBe("/simulation/runs/sim%2Fone/marks");
  });

  it("builds the list, detail, and action paths", () => {
    expect(runsPath()).toBe("/simulation/runs");
    expect(runsPath({ status: "marking", limit: 5 })).toBe(
      "/simulation/runs?status=marking&limit=5",
    );
    expect(runPath("sim/one")).toBe("/simulation/runs/sim%2Fone");
    expect(windowsPath("sim/one")).toBe("/simulation/runs/sim%2Fone/windows");
    expect(oraclePath("sim/one")).toBe("/simulation/runs/sim%2Fone/oracle");
    expect(checkPath("sim/one")).toBe("/simulation/runs/sim%2Fone/check");
    expect(decisionPath("sim/one")).toBe("/simulation/runs/sim%2Fone/decision");
  });
});

describe("canCreate", () => {
  it("accepts a complete request", () => {
    expect(canCreate(CREATE)).toBe(true);
  });

  it("rejects a blank field or an invalid seed", () => {
    expect(canCreate({ ...CREATE, premise: "  " })).toBe(false);
    expect(canCreate({ ...CREATE, bundle: "" })).toBe(false);
    expect(canCreate({ ...CREATE, inputs: "" })).toBe(false);
    expect(canCreate({ ...CREATE, criteria: "" })).toBe(false);
    expect(canCreate({ ...CREATE, seed: -1 })).toBe(false);
    expect(canCreate({ ...CREATE, seed: 1.5 })).toBe(false);
  });
});

describe("listRuns", () => {
  it("reads the summaries with the query filters", async () => {
    const seen = capture(200, [
      { sim_id: "sim", premise: "p", status: "draft", windows: 0, marks: 0 },
    ]);
    const rows = await listRuns({ status: "marking", limit: 5 });
    expect(seen[0]?.url).toBe(
      "/api/v1/simulation/runs?status=marking&limit=5",
    );
    expect(rows).toHaveLength(1);
    expect(rows[0]?.sim_id).toBe("sim");
  });
});

describe("createRun", () => {
  it("posts premise, bundle, inputs, seed, and criteria", async () => {
    const seen = capture(201, { sim_id: "sim_1" });
    const simId = await createRun(CREATE);
    expect(simId).toBe("sim_1");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs");
    expect(seen[0]?.init.method).toBe("POST");
    expect(JSON.parse(String(seen[0]?.init.body))).toEqual(CREATE);
  });

  it("rejects an acknowledgement without a sim id", async () => {
    capture(201, {});
    await expect(createRun(CREATE)).rejects.toThrow("without a sim_id");
  });
});

describe("fetchRun", () => {
  it("reads the detail", async () => {
    const seen = capture(200, RUN);
    const run = await fetchRun("sim");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim");
    expect(run.status).toBe("marking");
    expect(run.premise).toBe(RUN.premise);
  });
});

describe("selectWindows", () => {
  it("accepts a wrapped response and orders the windows", async () => {
    const seen = capture(200, {
      windows: [
        { index: 1, start: 100, end: 108 },
        { index: 0, start: 0, end: 8 },
      ],
    });
    const windows = await selectWindows("sim");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim/windows");
    expect(seen[0]?.init.method).toBe("POST");
    expect(windows.map((window) => window.index)).toEqual([0, 1]);
  });

  it("accepts a bare array and rejects a malformed window", async () => {
    capture(200, [{ index: 0, start: 0, end: 8 }]);
    expect(await selectWindows("sim")).toHaveLength(1);

    capture(200, [{ index: 0, start: 8, end: 0 }]);
    await expect(selectWindows("sim")).rejects.toThrow("invalid simulation window");
  });
});

describe("fetchSeries", () => {
  it("reads the window series", async () => {
    const body: ChartSeries = {
      sim_id: "sim",
      chart: 1,
      candles: [],
      ema_fast: [],
      ema_mid: [],
      ema_slow: [],
      panel: { indicator_code: 2, values: [] },
      events: [],
      levels: [],
    };
    const seen = capture(200, body);
    const series = await fetchSeries("sim", 1);
    expect(series.sim_id).toBe("sim");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim/windows/1/series");
  });
});

describe("fetchMarks", () => {
  it("normalizes the window identity and keeps the origin", async () => {
    const seen = capture(200, { marks: [RAW_MARK] });
    const marks = await fetchMarks("sim", "human");
    expect(seen[0]?.url).toBe(
      "/api/v1/simulation/runs/sim/marks?origin=human",
    );
    expect(marks[0]?.chart).toBe(1);
    expect(marks[0]?.origin).toBe("human");
  });

  it("accepts a bare array", async () => {
    capture(200, [RAW_MARK, { ...RAW_MARK, mark_id: "m2", origin: "oracle" }]);
    const marks = await fetchMarks("sim");
    expect(marks.map((mark) => mark.origin)).toEqual(["human", "oracle"]);
  });

  it("rejects a mark without an id or a numeric window", async () => {
    capture(200, [{ ...RAW_MARK, mark_id: "" }]);
    await expect(fetchMarks("sim")).rejects.toThrow("without a mark id");

    capture(200, [{ ...RAW_MARK, chart: "w1" }]);
    await expect(fetchMarks("sim")).rejects.toThrow("invalid window");
  });
});

describe("postMark", () => {
  it("posts the five mark fields and returns the id", async () => {
    const seen = capture(201, { mark_id: "m1" });
    const ack = await postMark("sim", DRAFT);
    expect(ack.mark_id).toBe("m1");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim/marks");
    expect(seen[0]?.init.method).toBe("POST");
    expect(JSON.parse(String(seen[0]?.init.body))).toEqual({
      chart: 1,
      bar: 17,
      action: "entry",
      quality: "good",
      side: "long",
      note: "higher low",
    });
  });

  it("surfaces the stable rejection code", async () => {
    capture(400, {
      error: { code: "simulation.mark_invalid", message: "bar outside the window" },
    });
    try {
      await postMark("sim", DRAFT);
      throw new Error("expected a failure");
    } catch (error) {
      expect(error).toBeInstanceOf(ApiError);
      expect((error as ApiError).code).toBe("simulation.mark_invalid");
    }
  });

  it("refuses a mark after the decision with the locked code", async () => {
    capture(409, {
      error: { code: "simulation.locked", message: "marking is closed" },
    });
    try {
      await postMark("sim", DRAFT);
      throw new Error("expected a failure");
    } catch (error) {
      expect(error).toBeInstanceOf(ApiError);
      expect((error as ApiError).code).toBe("simulation.locked");
    }
  });

  it("rejects an acknowledgement without a mark id", async () => {
    capture(201, {});
    await expect(postMark("sim", DRAFT)).rejects.toThrow("without a mark id");
  });
});

describe("deleteMark", () => {
  it("accepts the 204 no-content response", async () => {
    const seen = capture(204, undefined);
    await expect(deleteMark("sim", "m/1")).resolves.toBeUndefined();
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim/marks/m%2F1");
    expect(seen[0]?.init.method).toBe("DELETE");
  });

  it("surfaces a rejection with a stable code", async () => {
    capture(409, {
      error: { code: "simulation.locked", message: "marking is closed" },
    });
    try {
      await deleteMark("sim", "m1");
      throw new Error("expected a failure");
    } catch (error) {
      expect(error).toBeInstanceOf(ApiError);
      expect((error as ApiError).code).toBe("simulation.locked");
    }
  });
});

describe("postOracle", () => {
  it("posts the oracle request and normalizes the marks", async () => {
    const seen = capture(200, {
      marks: [{ ...RAW_MARK, mark_id: "o1", origin: "oracle", quality: "neutral" }],
    });
    const marks = await postOracle("sim");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim/oracle");
    expect(seen[0]?.init.method).toBe("POST");
    expect(marks[0]?.origin).toBe("oracle");
    expect(marks[0]?.quality).toBe("neutral");
  });
});

describe("postCheck", () => {
  it("reads a direct check and its verdict", async () => {
    const seen = capture(200, CHECK);
    const check = await postCheck("sim");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim/check");
    expect(check.passed).toBe(true);
    expect(check.events).toBe(42);
  });

  it("accepts a wrapped check and refuses a missing verdict", async () => {
    capture(200, { check: { ...CHECK, passed: false, failures: ["unstable"] } });
    const check = await postCheck("sim");
    expect(check.passed).toBe(false);
    expect(check.failures).toEqual(["unstable"]);

    capture(200, {});
    await expect(postCheck("sim")).rejects.toThrow("no check verdict");
  });

  it("surfaces an incomplete check with its stable code", async () => {
    capture(409, {
      error: {
        code: "simulation.check_incomplete",
        message: "fewer than 30 events",
      },
    });
    try {
      await postCheck("sim");
      throw new Error("expected a failure");
    } catch (error) {
      expect(error).toBeInstanceOf(ApiError);
      expect((error as ApiError).code).toBe("simulation.check_incomplete");
    }
  });
});

describe("postDecision", () => {
  it("posts the decision and note and returns the record", async () => {
    const seen = capture(201, {
      decision: "test",
      note: "the boundary holds",
      decided_at_utc: "2026-11-14T12:00:00Z",
    });
    const record = await postDecision("sim", "test", "the boundary holds");
    expect(seen[0]?.url).toBe("/api/v1/simulation/runs/sim/decision");
    expect(seen[0]?.init.method).toBe("POST");
    expect(JSON.parse(String(seen[0]?.init.body))).toEqual({
      decision: "test",
      note: "the boundary holds",
    });
    expect(record.decision).toBe("test");
  });

  it("accepts a wrapped record and refuses an unknown decision", async () => {
    capture(201, {
      decision: { decision: "refine", note: "one condition only" },
    });
    const record = await postDecision("sim", "refine", "one condition only");
    expect(record.decision).toBe("refine");

    capture(201, { decision: "maybe", note: "not a decision" });
    await expect(postDecision("sim", "test", "note")).rejects.toThrow(
      "no decision record",
    );
  });
});
