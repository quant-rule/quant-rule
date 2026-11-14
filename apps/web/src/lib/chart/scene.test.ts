import { describe, expect, it } from "bun:test";
import { buildChartScene } from "./scene";
import type { ChartMark, ChartSeries } from "./types";

const SERIES: ChartSeries = {
  sim_id: "sim_fixture",
  chart: 1,
  candles: [
    { bar: 0, open: 100, high: 102, low: 99, close: 101, volume: 10 },
    { bar: 1, open: 101, high: 103, low: 100, close: 100, volume: 20 },
    { bar: 2, open: 100, high: 101, low: 98, close: 99, volume: 0 },
    { bar: 3, open: 99, high: 102, low: 98, close: 102, volume: 30 },
  ],
  ema_fast: [null, 101, 100.5, 101],
  ema_mid: [null, null, 100.25, 100.75],
  ema_slow: [null, null, null, 100.5],
  panel: { indicator_code: 2, values: [null, 50, 40, 60] },
  events: [
    {
      event_code: 8,
      event_type: 3,
      event_subtype_code: 1,
      source_code: 4,
      occurred_bar_index: 0,
      available_bar_index: 2,
      direction_code: 1,
    },
  ],
  levels: [
    {
      reference_code: 1001,
      scale_code: 1,
      kind_code: -1,
      price: 98.5,
      occurred_bar_index: 0,
      available_bar_index: 1,
    },
  ],
};

function mark(): ChartMark {
  return {
    mark_id: "m1",
    chart: 1,
    bar: 1,
    action: "entry",
    quality: "good",
    side: "long",
    origin: "human",
    note: "",
  };
}

describe("buildChartScene", () => {
  it("renders the same path data for the same series", () => {
    const first = buildChartScene(SERIES, []);
    const second = buildChartScene(SERIES, []);
    expect(first).toEqual(second);
  });

  it("keeps the candle, indicator, event, and level geometry when marks are added", () => {
    const without = buildChartScene(SERIES, []);
    const withMark = buildChartScene(SERIES, [mark()]);
    expect(withMark.candles).toEqual(without.candles);
    expect(withMark.volume).toEqual(without.volume);
    expect(withMark.ema).toEqual(without.ema);
    expect(withMark.panel).toEqual(without.panel);
    expect(withMark.events).toEqual(without.events);
    expect(withMark.levels).toEqual(without.levels);
  });

  it("draws the fixed series as checked-in golden path data", () => {
    const scene = buildChartScene(SERIES, []);
    expect(scene.candles.wicks).toBe(
      "M174.00 103.60L174.00 290.40M394.00 41.34L394.00 228.13M614.00 165.87L614.00 352.66M834.00 103.60L834.00 352.66",
    );
    expect(scene.candles.up).toBe(
      "M105.80 165.87h136.40v62.26h-136.40ZM765.80 103.60h136.40v186.79h-136.40Z",
    );
    expect(scene.candles.down).toBe(
      "M325.80 165.87h136.40v62.26h-136.40ZM545.80 228.13h136.40v62.26h-136.40Z",
    );
    expect(scene.ema.fast).toBe("M394.00 165.87L614.00 197.00L834.00 165.87");
    expect(scene.ema.mid).toBe("M614.00 212.57L834.00 181.43");
    expect(scene.ema.slow).toBe("M834.00 197.00");
    expect(scene.panel.path).toBe("M394.00 488.00L614.00 527.66L834.00 448.34");
    expect(scene.panel.title).toBe("Relative strength index");
  });

  it("draws volume bars only for positive volume", () => {
    const scene = buildChartScene(SERIES, []);
    expect(scene.volume.up).toBe(
      "M105.80 412.67h136.40v21.33h-136.40ZM765.80 370.00h136.40v64.00h-136.40Z",
    );
    expect(scene.volume.down).toBe("M325.80 391.33h136.40v42.67h-136.40Z");
    expect(scene.volume.up).not.toContain("614");
    expect(scene.volume.down).not.toContain("614");
  });

  it("places an event at the bar it became knowable, not the bar it occurred", () => {
    const scene = buildChartScene(SERIES, []);
    expect(scene.events).toHaveLength(1);
    expect(scene.events[0]?.bar).toBe(2);
    expect(scene.events[0]?.x).toBe(614);
    expect(scene.events[0]?.name).toBe("signal.rsi_threshold");
    expect(scene.events[0]?.detail).toBe("Signal · threshold crossing · RSI");
  });

  it("starts a confirmed level at its availability bar", () => {
    const scene = buildChartScene(SERIES, []);
    expect(scene.levels).toHaveLength(1);
    expect(scene.levels[0]?.x1).toBe(394);
    expect(scene.levels[0]?.x2).toBe(944);
    expect(scene.levels[0]?.label).toBe("Level 1001 · minor · low");
  });

  it("drops a level that is not yet knowable in the window", () => {
    const future: ChartSeries = {
      ...SERIES,
      levels: [
        {
          reference_code: 1002,
          scale_code: 2,
          kind_code: 1,
          price: 104,
          occurred_bar_index: 2,
          available_bar_index: 9,
        },
      ],
    };
    expect(buildChartScene(future, []).levels).toEqual([]);
  });

  it("draws a mark at the bar with its class label", () => {
    const scene = buildChartScene(SERIES, [mark()]);
    expect(scene.marks).toHaveLength(1);
    expect(scene.marks[0]?.markId).toBe("m1");
    expect(scene.marks[0]?.x).toBe(394);
    expect(scene.marks[0]?.y).toBeCloseTo(238.13, 2);
    expect(scene.marks[0]?.label).toBe("Entry · Good · Long");
    expect(scene.marks[0]?.path).toBe("M394.00 232.13L398.00 242.13L390.00 242.13Z");
  });

  it("ignores a mark outside the window", () => {
    expect(buildChartScene(SERIES, [{ ...mark(), bar: 99 }]).marks).toEqual([]);
  });

  it("renders an empty series without inventing a value", () => {
    const empty: ChartSeries = {
      ...SERIES,
      candles: [],
      ema_fast: [],
      ema_mid: [],
      ema_slow: [],
      panel: { indicator_code: 2, values: [] },
      events: [],
      levels: [],
    };
    const scene = buildChartScene(empty, []);
    expect(scene.barCount).toBe(0);
    expect(scene.candles).toEqual({ wicks: "", up: "", down: "" });
    expect(scene.volume).toEqual({ up: "", down: "" });
    expect(scene.ema).toEqual({ fast: "", mid: "", slow: "" });
    expect(scene.panel.path).toBe("");
    expect(scene.events).toEqual([]);
    expect(scene.levels).toEqual([]);
    expect(scene.marks).toEqual([]);
  });

  it("formats a flat price domain without dividing by zero", () => {
    const flat: ChartSeries = {
      ...SERIES,
      candles: [{ bar: 0, open: 100, high: 100, low: 100, close: 100, volume: 1 }],
      ema_fast: [100],
      ema_mid: [100],
      ema_slow: [100],
      panel: { indicator_code: 2, values: [50] },
      events: [],
      levels: [],
    };
    const scene = buildChartScene(flat, []);
    expect(scene.candles.wicks).toContain("M504.00");
    expect(Number.isFinite(scene.priceTicks[0]?.y ?? Number.NaN)).toBe(true);
    expect(scene.candles.wicks).not.toContain("NaN");
  });
});
