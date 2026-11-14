import { describe, expect, it } from "bun:test";
import {
  eventDetail,
  eventName,
  eventSubtype,
  eventType,
  indicatorName,
  scaleName,
  sourceName,
} from "./labels";
import type { ChartEvent } from "./types";

function event(overrides: Partial<ChartEvent> = {}): ChartEvent {
  return {
    event_code: 8,
    event_type: 3,
    event_subtype_code: 1,
    source_code: 4,
    occurred_bar_index: 2,
    available_bar_index: 3,
    direction_code: 1,
    ...overrides,
  };
}

describe("chart labels", () => {
  it("maps an event code through the generated table", () => {
    expect(eventName(8)).toBe("signal.rsi_threshold");
    expect(eventType(3)).toBe("Signal");
    expect(sourceName(4)).toBe("RSI");
    expect(eventSubtype(event())).toBe("threshold crossing");
  });

  it("selects the subtype table that belongs to the event type", () => {
    expect(eventSubtype(event({ event_type: 1, event_subtype_code: 2 }))).toBe("cross");
    expect(eventSubtype(event({ event_type: 2, event_subtype_code: 3 }))).toBe(
      "hidden bullish",
    );
  });

  it("names the one indicator panel", () => {
    expect(indicatorName(2)).toBe("Relative strength index");
    expect(scaleName(1)).toBe("minor");
  });

  it("never guesses an unknown code", () => {
    expect(eventName(9_999)).toBe("unknown(9999)");
    expect(eventType(9)).toBe("unknown(9)");
    expect(sourceName(9_999)).toBe("unknown(9999)");
    expect(eventSubtype(event({ event_type: 9 }))).toBe("unknown(1)");
    expect(eventDetail(event({ event_type: 9 }))).toBe(
      "unknown(9) · unknown(1) · RSI",
    );
  });
});
