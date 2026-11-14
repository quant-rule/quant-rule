/**
 * Deterministic SVG path builders.
 *
 * Every coordinate is rounded to two decimals with `toFixed`, so the same
 * series produces the same path data byte for byte. A missing value breaks the
 * line rather than becoming zero; a gap in an EMA is a gap, not a value.
 */

import type { ChartCandle } from "./types";

/** One path coordinate, fixed to two decimals and without a negative zero. */
export function coordinate(value: number): string {
  if (!Number.isFinite(value)) {
    return "0.00";
  }
  const rounded = Math.round(value * 100) / 100;
  const normal = Object.is(rounded, -0) ? 0 : rounded;
  return normal.toFixed(2);
}

/**
 * A polyline path over a nullable series.
 *
 * A null or non-finite value ends the current subpath, so a later value starts
 * a new `M` instead of drawing through a measurement that was never published.
 */
export function linePath(
  values: readonly (number | null)[],
  xOf: (index: number) => number,
  yOf: (value: number) => number,
): string {
  let path = "";
  let drawing = false;
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === null || value === undefined || !Number.isFinite(value)) {
      drawing = false;
      continue;
    }
    path += `${drawing ? "L" : "M"}${coordinate(xOf(index))} ${coordinate(yOf(value))}`;
    drawing = true;
  }
  return path;
}

/** The three candle paths: one stroke for wicks and one fill per direction. */
export interface CandlePaths {
  wicks: string;
  up: string;
  down: string;
}

/**
 * Build candle wicks and bodies.
 *
 * A body is at least one pixel tall so a doji remains visible, and a bar with
 * equal open and close still has one deterministic body. The half-width is a
 * fraction of the slot with a one-pixel floor, so the narrowest window still
 * draws a body.
 */
export function candlePaths(
  candles: readonly ChartCandle[],
  xOf: (index: number) => number,
  yOf: (value: number) => number,
  slotWidth: number,
): CandlePaths {
  const halfWidth = Math.max(0.5, slotWidth * 0.31);
  let wicks = "";
  let up = "";
  let down = "";
  for (let index = 0; index < candles.length; index += 1) {
    const candle = candles[index];
    if (candle === undefined) {
      continue;
    }
    const values = [candle.open, candle.high, candle.low, candle.close];
    if (!values.every((value) => Number.isFinite(value))) {
      continue;
    }
    const x = xOf(index);
    wicks += `M${coordinate(x)} ${coordinate(yOf(candle.high))}L${coordinate(x)} ${coordinate(yOf(candle.low))}`;

    const openY = yOf(candle.open);
    const closeY = yOf(candle.close);
    const top = Math.min(openY, closeY);
    const bottom = Math.max(openY, closeY);
    const rawHeight = bottom - top;
    const bodyTop = rawHeight < 1 ? (top + bottom) / 2 - 0.5 : top;
    const bodyHeight = Math.max(1, rawHeight);
    const body = `M${coordinate(x - halfWidth)} ${coordinate(bodyTop)}h${coordinate(halfWidth * 2)}v${coordinate(bodyHeight)}h${coordinate(-halfWidth * 2)}Z`;
    if (candle.close >= candle.open) {
      up += body;
    } else {
      down += body;
    }
  }
  return { wicks, up, down };
}

/** The two volume paths, split by candle direction. */
export interface VolumePaths {
  up: string;
  down: string;
}

/**
 * Build volume bars from zero.
 *
 * A missing or non-positive volume draws nothing. A zero volume is a valid
 * value, but a zero-height bar would make every zero look like a one-pixel
 * trade, so it is left empty.
 */
export function volumePaths(
  candles: readonly ChartCandle[],
  xOf: (index: number) => number,
  yOf: (value: number) => number,
  slotWidth: number,
): VolumePaths {
  const halfWidth = Math.max(0.5, slotWidth * 0.31);
  const base = yOf(0);
  let up = "";
  let down = "";
  for (let index = 0; index < candles.length; index += 1) {
    const candle = candles[index];
    if (candle === undefined || candle.volume === null) {
      continue;
    }
    if (!Number.isFinite(candle.volume) || candle.volume <= 0) {
      continue;
    }
    const x = xOf(index);
    const top = Math.min(base, yOf(candle.volume));
    const height = Math.max(1, Math.abs(base - top));
    const bar = `M${coordinate(x - halfWidth)} ${coordinate(top)}h${coordinate(halfWidth * 2)}v${coordinate(height)}h${coordinate(-halfWidth * 2)}Z`;
    if (candle.close >= candle.open) {
      up += bar;
    } else {
      down += bar;
    }
  }
  return { up, down };
}

/** A small triangle arrow used by the mark layer and signal markers. */
export function arrowPath(x: number, y: number, up: boolean): string {
  const tip = up ? y - 6 : y + 6;
  const base = up ? y + 4 : y - 4;
  return `M${coordinate(x)} ${coordinate(tip)}L${coordinate(x + 4)} ${coordinate(base)}L${coordinate(x - 4)} ${coordinate(base)}Z`;
}

/** A diamond used by divergence markers. */
export function diamondPath(x: number, y: number): string {
  return `M${coordinate(x)} ${coordinate(y - 4)}L${coordinate(x + 4)} ${coordinate(y)}L${coordinate(x)} ${coordinate(y + 4)}L${coordinate(x - 4)} ${coordinate(y)}Z`;
}
