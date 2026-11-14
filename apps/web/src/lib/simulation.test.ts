import { afterEach, describe, expect, it } from "bun:test";
import { ApiError } from "./api";
import {
  deleteMark,
  fetchSeries,
  marksPath,
  postMark,
  seriesPath,
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

describe("simulation paths", () => {
  it("encodes the simulation id and chart number", () => {
    expect(seriesPath("sim/one", 2)).toBe(
      "/simulation/runs/sim%2Fone/windows/2/series",
    );
    expect(marksPath("sim/one")).toBe("/simulation/runs/sim%2Fone/marks");
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
      expect((error as ApiError).code).toBe("simulation.locked");
    }
  });
});
