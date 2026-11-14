/**
 * Build one deterministic chart scene from a series and its marks.
 *
 * The scene is pure data: paths, coordinates, and words. It reads no DOM,
 * clock, or randomness, so a fixed series always yields the same scene and the
 * Svelte layer only lays it out. Marks are an overlay; they never alter a
 * candle, indicator, event, or level coordinate.
 */

import { eventDetail, eventName, indicatorName, scaleName } from "./labels";
import { markClassLabel } from "./markClasses";
import {
  arrowPath,
  candlePaths,
  linePath,
  volumePaths,
  type CandlePaths,
  type VolumePaths,
} from "./paths";
import {
  barCenter,
  barSlot,
  DEFAULT_LAYOUT,
  regions,
  verticalScale,
  type ChartLayout,
  type ChartRegions,
} from "./scale";
import type { ChartMark, ChartSeries } from "./types";

/** One horizontal price grid line and its axis label. */
export interface PriceTick {
  key: string;
  price: number;
  y: number;
  label: string;
}

/** The marker shape an event type selects. */
export type EventMarkerKind = "level" | "divergence" | "signal" | "unknown";

/** One event marker in the lane above the price plot. */
export interface EventMarker {
  key: string;
  bar: number;
  kind: EventMarkerKind;
  x: number;
  y: number;
  direction: number;
  name: string;
  detail: string;
}

/** One confirmed level, drawn from the bar it became knowable to the right edge. */
export interface LevelLine {
  key: string;
  referenceCode: number;
  x1: number;
  x2: number;
  y: number;
  scaleCode: number;
  kindCode: number;
  label: string;
}

/** One mark arrow on the price plot. */
export interface MarkMarker {
  key: string;
  markId: string;
  x: number;
  y: number;
  up: boolean;
  action: string;
  quality: string;
  side: string;
  label: string;
  path: string;
}

/** The one indicator panel under the volume. */
export interface PanelView {
  title: string;
  path: string;
  minLabel: string;
  maxLabel: string;
  zeroY: number | null;
}

/** Everything the chart component renders. */
export interface ChartScene {
  regions: ChartRegions;
  slotWidth: number;
  barCount: number;
  priceTicks: PriceTick[];
  candles: CandlePaths;
  volume: VolumePaths;
  ema: { fast: string; mid: string; slow: string };
  panel: PanelView;
  events: EventMarker[];
  levels: LevelLine[];
  marks: MarkMarker[];
}

interface Domain {
  low: number;
  high: number;
}

function finiteDomain(values: readonly (number | null)[]): Domain | null {
  let low = Number.POSITIVE_INFINITY;
  let high = Number.NEGATIVE_INFINITY;
  for (const value of values) {
    if (value === null || value === undefined || !Number.isFinite(value)) {
      continue;
    }
    low = Math.min(low, value);
    high = Math.max(high, value);
  }
  return Number.isFinite(low) && Number.isFinite(high) ? { low, high } : null;
}

/**
 * The visible price range.
 *
 * Levels and EMAs are part of the range because they are drawn against price;
 * an EMA above every candle would otherwise leave the plot. A flat domain gets
 * a small absolute pad so the line sits in the middle rather than on an edge.
 */
function priceDomain(series: ChartSeries): Domain {
  let low = Number.POSITIVE_INFINITY;
  let high = Number.NEGATIVE_INFINITY;
  for (const candle of series.candles) {
    if (Number.isFinite(candle.low)) {
      low = Math.min(low, candle.low);
    }
    if (Number.isFinite(candle.high)) {
      high = Math.max(high, candle.high);
    }
  }
  for (const level of series.levels) {
    if (level.available_bar_index >= 0 && level.available_bar_index < series.candles.length) {
      if (Number.isFinite(level.price)) {
        low = Math.min(low, level.price);
        high = Math.max(high, level.price);
      }
    }
  }
  for (const values of [series.ema_fast, series.ema_mid, series.ema_slow]) {
    for (const value of values.slice(0, series.candles.length)) {
      if (value !== null && value !== undefined && Number.isFinite(value)) {
        low = Math.min(low, value);
        high = Math.max(high, value);
      }
    }
  }
  if (!Number.isFinite(low) || !Number.isFinite(high)) {
    return { low: 0, high: 1 };
  }
  const span = high - low;
  const pad = span > 0 ? span * 0.03 : Math.max(Math.abs(high), 1) * 0.005;
  return { low: low - pad, high: high + pad };
}

function panelDomain(values: readonly (number | null)[]): Domain | null {
  const domain = finiteDomain(values);
  if (domain === null) {
    return null;
  }
  const span = domain.high - domain.low;
  const pad = span > 0 ? span * 0.08 : Math.max(Math.abs(domain.high), 1) * 0.05;
  return { low: domain.low - pad, high: domain.high + pad };
}

function eventKind(eventType: number): EventMarkerKind {
  switch (eventType) {
    case 1:
      return "level";
    case 2:
      return "divergence";
    case 3:
      return "signal";
    default:
      return "unknown";
  }
}

function eventMarkerY(area: ChartRegions, eventType: number): number {
  const row = Math.min(3, Math.max(1, eventType)) - 1;
  return area.eventTop + 8 + row * 6;
}

function levelKind(kindCode: number): string {
  if (kindCode === 1) {
    return "high";
  }
  if (kindCode === -1) {
    return "low";
  }
  return "level";
}

function priceLabels(domain: Domain, yOf: (value: number) => number): PriceTick[] {
  const ticks: PriceTick[] = [];
  const steps = 4;
  for (let index = 0; index <= steps; index += 1) {
    const price = domain.high - ((domain.high - domain.low) * index) / steps;
    ticks.push({
      key: `price-${index}`,
      price,
      y: yOf(price),
      label: price.toFixed(2),
    });
  }
  return ticks;
}

/**
 * Build the scene for one series and its visible marks.
 *
 * The layout is a parameter so a test can use a small, hand-checkable viewBox;
 * production uses [`DEFAULT_LAYOUT`].
 */
export function buildChartScene(
  series: ChartSeries,
  marks: readonly ChartMark[],
  layout: ChartLayout = DEFAULT_LAYOUT,
): ChartScene {
  const area = regions(layout);
  const count = series.candles.length;
  const slotWidth = barSlot(layout, count);
  const xOf = (index: number) => barCenter(layout, count, index);
  const domain = priceDomain(series);
  const yOf = verticalScale(domain.low, domain.high, area.priceTop, area.priceHeight);

  const candleData = candlePaths(series.candles, xOf, yOf, slotWidth);

  const maxVolume = series.candles.reduce((max, candle) => {
    if (candle.volume === null || !Number.isFinite(candle.volume)) {
      return max;
    }
    return Math.max(max, candle.volume);
  }, 0);
  const volumeY = verticalScale(0, maxVolume > 0 ? maxVolume : 1, area.volumeTop, area.volumeHeight);
  const volumeData = volumePaths(series.candles, xOf, volumeY, slotWidth);

  const panelValues = series.panel.values.slice(0, count);
  const panelScale = panelDomain(panelValues);
  let panel: PanelView;
  if (panelScale === null) {
    panel = {
      title: indicatorName(series.panel.indicator_code),
      path: "",
      minLabel: "—",
      maxLabel: "—",
      zeroY: null,
    };
  } else {
    const panelY = verticalScale(
      panelScale.low,
      panelScale.high,
      area.panelTop,
      area.panelHeight,
    );
    const crossesZero = panelScale.low <= 0 && panelScale.high >= 0;
    panel = {
      title: indicatorName(series.panel.indicator_code),
      path: linePath(panelValues, xOf, panelY),
      minLabel: panelScale.low.toFixed(2),
      maxLabel: panelScale.high.toFixed(2),
      zeroY: crossesZero ? panelY(0) : null,
    };
  }

  const events: EventMarker[] = [];
  for (let index = 0; index < series.events.length; index += 1) {
    const event = series.events[index];
    if (event === undefined) {
      continue;
    }
    const bar = event.available_bar_index;
    if (!Number.isInteger(bar) || bar < 0 || bar >= count) {
      continue;
    }
    events.push({
      key: `event-${index}-${event.event_code}-${bar}`,
      bar,
      kind: eventKind(event.event_type),
      x: xOf(bar),
      y: eventMarkerY(area, event.event_type),
      direction: Math.sign(event.direction_code),
      name: eventName(event.event_code),
      detail: eventDetail(event),
    });
  }

  const levels: LevelLine[] = [];
  for (let index = 0; index < series.levels.length; index += 1) {
    const level = series.levels[index];
    if (level === undefined || !Number.isFinite(level.price)) {
      continue;
    }
    if (!Number.isInteger(level.available_bar_index) || level.available_bar_index < 0) {
      continue;
    }
    if (level.available_bar_index >= count) {
      continue;
    }
    const start = Math.max(0, level.available_bar_index);
    levels.push({
      key: `level-${index}-${level.reference_code}`,
      referenceCode: level.reference_code,
      x1: xOf(start),
      x2: area.left + area.plotWidth,
      y: yOf(level.price),
      scaleCode: level.scale_code,
      kindCode: level.kind_code,
      label: `Level ${level.reference_code} · ${scaleName(level.scale_code)} · ${levelKind(level.kind_code)}`,
    });
  }

  const visibleMarks: MarkMarker[] = [];
  for (let index = 0; index < marks.length; index += 1) {
    const mark = marks[index];
    if (mark === undefined || !Number.isInteger(mark.bar) || mark.bar < 0 || mark.bar >= count) {
      continue;
    }
    const candle = series.candles[mark.bar];
    if (candle === undefined) {
      continue;
    }
    const up =
      (mark.action === "entry" && mark.side === "long") ||
      (mark.action === "exit" && mark.side === "short");
    const rawY = up ? yOf(candle.low) + 10 : yOf(candle.high) - 10;
    const y = Math.min(area.priceBottom - 6, Math.max(area.priceTop + 6, rawY));
    const x = xOf(mark.bar);
    visibleMarks.push({
      key: `mark-${mark.mark_id}-${index}`,
      markId: mark.mark_id,
      x,
      y,
      up,
      action: mark.action,
      quality: mark.quality,
      side: mark.side,
      label: markClassLabel(mark),
      path: arrowPath(x, y, up),
    });
  }

  return {
    regions: area,
    slotWidth,
    barCount: count,
    priceTicks: priceLabels(domain, yOf),
    candles: candleData,
    volume: volumeData,
    ema: {
      fast: linePath(series.ema_fast.slice(0, count), xOf, yOf),
      mid: linePath(series.ema_mid.slice(0, count), xOf, yOf),
      slow: linePath(series.ema_slow.slice(0, count), xOf, yOf),
    },
    panel,
    events,
    levels,
    marks: visibleMarks,
  };
}
