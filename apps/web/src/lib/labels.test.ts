import { describe, expect, it } from "bun:test";
import {
  DIVERGENCE_SUBTYPES,
  EVENTS,
  EVENT_TYPES,
  INDICATORS,
  LEVEL_SUBTYPES,
  ROLES,
  SIGNAL_SUBTYPES,
  SOURCES,
} from "./labels.generated";

const tables = {
  DIVERGENCE_SUBTYPES,
  EVENTS,
  EVENT_TYPES,
  INDICATORS,
  LEVEL_SUBTYPES,
  ROLES,
  SIGNAL_SUBTYPES,
  SOURCES,
};

describe("the generated label table", () => {
  it("maps every code to a non-empty word", () => {
    for (const [name, table] of Object.entries(tables)) {
      const entries = Object.entries(table);
      expect(entries.length, name).toBeGreaterThan(0);
      for (const [code, label] of entries) {
        expect(Number.isInteger(Number(code)), `${name} ${code}`).toBe(true);
        expect(Number(code)).toBeGreaterThan(0);
        expect(label.length, `${name} ${code}`).toBeGreaterThan(0);
      }
    }
  });

  it("carries the shared event type, role, and source words", () => {
    expect(EVENT_TYPES[1]).toBe("Level");
    expect(ROLES[1]).toBe("signal");
    expect(SOURCES[4]).toBe("RSI");
  });

  it("has one label per registered event and indicator code", () => {
    expect(Object.keys(EVENTS).length).toBe(150);
    expect(Object.keys(INDICATORS).length).toBe(49);
  });
});
