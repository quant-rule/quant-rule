import { describe, expect, it } from "bun:test";
import {
  barAtPointer,
  barCenter,
  barSlot,
  DEFAULT_LAYOUT,
  regions,
  verticalScale,
  viewBoxX,
} from "./scale";

const area = regions(DEFAULT_LAYOUT);

describe("barAtPointer", () => {
  it("maps the left edge to the first bar", () => {
    expect(barAtPointer(area.left, DEFAULT_LAYOUT, 8)).toBe(0);
  });

  it("maps the right edge to the last bar", () => {
    expect(barAtPointer(area.left + area.plotWidth, DEFAULT_LAYOUT, 8)).toBe(7);
  });

  it("maps fractional pixels inside a slot to that slot", () => {
    const slot = barSlot(DEFAULT_LAYOUT, 8);
    expect(barAtPointer(area.left + slot * 0.5, DEFAULT_LAYOUT, 8)).toBe(0);
    expect(barAtPointer(area.left + slot * 1.5, DEFAULT_LAYOUT, 8)).toBe(1);
    expect(barAtPointer(area.left + slot * 6.5, DEFAULT_LAYOUT, 8)).toBe(6);
  });

  it("puts the slot boundary on the next bar", () => {
    const slot = barSlot(DEFAULT_LAYOUT, 8);
    expect(barAtPointer(area.left + slot - 0.001, DEFAULT_LAYOUT, 8)).toBe(0);
    expect(barAtPointer(area.left + slot, DEFAULT_LAYOUT, 8)).toBe(1);
  });

  it("works with a fractional slot width", () => {
    const slot = barSlot(DEFAULT_LAYOUT, 7);
    expect(slot).toBeCloseTo(area.plotWidth / 7, 12);
    expect(barAtPointer(area.left + slot * 2.25, DEFAULT_LAYOUT, 7)).toBe(2);
    expect(barAtPointer(area.left + slot * 6.99, DEFAULT_LAYOUT, 7)).toBe(6);
    expect(barAtPointer(area.left + area.plotWidth - 0.001, DEFAULT_LAYOUT, 7)).toBe(6);
  });

  it("selects no bar outside the plot", () => {
    expect(barAtPointer(area.left - 0.01, DEFAULT_LAYOUT, 8)).toBeNull();
    expect(barAtPointer(area.left + area.plotWidth + 0.01, DEFAULT_LAYOUT, 8)).toBeNull();
  });

  it("selects no bar for an empty or non-finite window", () => {
    expect(barAtPointer(area.left, DEFAULT_LAYOUT, 0)).toBeNull();
    expect(barAtPointer(Number.NaN, DEFAULT_LAYOUT, 8)).toBeNull();
  });
});

describe("barCenter", () => {
  it("centres the first and last bar inside the plot", () => {
    expect(barCenter(DEFAULT_LAYOUT, 8, 0)).toBe(area.left + (area.plotWidth / 8) * 0.5);
    expect(barCenter(DEFAULT_LAYOUT, 8, 7)).toBe(area.left + (area.plotWidth / 8) * 7.5);
  });
});

describe("verticalScale", () => {
  it("maps the high to the top and the low to the bottom", () => {
    const scale = verticalScale(90, 110, 20, 200);
    expect(scale(110)).toBe(20);
    expect(scale(90)).toBe(220);
    expect(scale(100)).toBe(120);
  });

  it("centres a zero-span domain instead of dividing by zero", () => {
    const scale = verticalScale(100, 100, 20, 200);
    expect(scale(100)).toBe(120);
    expect(Number.isFinite(scale(100))).toBe(true);
  });
});

describe("viewBoxX", () => {
  it("keeps the viewBox coordinate at the native size", () => {
    expect(viewBoxX(120, 0, 960, 960)).toBe(120);
  });

  it("scales the client coordinate to the viewBox", () => {
    expect(viewBoxX(10, 0, 480, 960)).toBe(20);
    expect(viewBoxX(10.5, 10, 100, 960)).toBeCloseTo(4.8, 12);
  });

  it("rejects a zero-width rectangle", () => {
    expect(viewBoxX(10, 0, 0, 960)).toBeNull();
  });
});
