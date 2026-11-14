/**
 * The chart's fixed geometry and pointer-to-bar mapping.
 *
 * Every coordinate is derived from the layout and the bar count, never from a
 * measured DOM element. The same series and layout therefore produce the same
 * viewBox coordinates at every size, and the pointer conversion is the only
 * place that reads a client rectangle.
 */

/** The chart's fixed viewBox geometry, in CSS pixels. */
export interface ChartLayout {
  width: number;
  /** The price plot's left edge; the axis labels sit to its left. */
  left: number;
  /** The right margin after the plot. */
  right: number;
  top: number;
  /** The lane above the price plot that carries event markers. */
  eventLane: number;
  priceHeight: number;
  gap: number;
  volumeHeight: number;
  panelHeight: number;
  bottom: number;
}

/** One layout that keeps every region aligned and deterministic. */
export const DEFAULT_LAYOUT: ChartLayout = {
  width: 960,
  left: 64,
  right: 16,
  top: 8,
  eventLane: 24,
  priceHeight: 330,
  gap: 8,
  volumeHeight: 64,
  panelHeight: 92,
  bottom: 24,
};

/** The resolved vertical regions of the chart. */
export interface ChartRegions {
  width: number;
  height: number;
  left: number;
  plotWidth: number;
  eventTop: number;
  eventBottom: number;
  priceTop: number;
  priceBottom: number;
  priceHeight: number;
  volumeTop: number;
  volumeBottom: number;
  volumeHeight: number;
  panelTop: number;
  panelBottom: number;
  panelHeight: number;
}

/** Resolve the layout into the shared vertical regions. */
export function regions(layout: ChartLayout): ChartRegions {
  const plotWidth = Math.max(0, layout.width - layout.left - layout.right);
  const eventTop = layout.top;
  const eventBottom = eventTop + layout.eventLane;
  const priceTop = eventBottom;
  const priceBottom = priceTop + layout.priceHeight;
  const volumeTop = priceBottom + layout.gap;
  const volumeBottom = volumeTop + layout.volumeHeight;
  const panelTop = volumeBottom + layout.gap;
  const panelBottom = panelTop + layout.panelHeight;
  return {
    width: layout.width,
    height: panelBottom + layout.bottom,
    left: layout.left,
    plotWidth,
    eventTop,
    eventBottom,
    priceTop,
    priceBottom,
    priceHeight: layout.priceHeight,
    volumeTop,
    volumeBottom,
    volumeHeight: layout.volumeHeight,
    panelTop,
    panelBottom,
    panelHeight: layout.panelHeight,
  };
}

/**
 * A top-down linear scale.
 *
 * A zero-span domain puts the value in the middle instead of dividing by zero,
 * so a flat window still renders a stable line.
 */
export function verticalScale(
  low: number,
  high: number,
  top: number,
  height: number,
): (value: number) => number {
  const span = high - low;
  if (!Number.isFinite(span) || span <= 0) {
    return () => top + height / 2;
  }
  return (value: number) => top + ((high - value) / span) * height;
}

/** The width of one bar's slot. */
export function barSlot(layout: ChartLayout, count: number): number {
  if (count <= 0) {
    return 0;
  }
  return regions(layout).plotWidth / count;
}

/** The horizontal centre of one bar. */
export function barCenter(layout: ChartLayout, count: number, index: number): number {
  const slot = barSlot(layout, count);
  return regions(layout).left + (index + 0.5) * slot;
}

/**
 * Map a viewBox x coordinate to a bar index.
 *
 * The plot is half-open per bar and inclusive at the right edge: a pointer
 * exactly on the right edge still belongs to the last bar, because that is the
 * only bar under it. A pointer outside the plot selects no bar.
 */
export function barAtPointer(
  x: number,
  layout: ChartLayout,
  count: number,
): number | null {
  if (!Number.isFinite(x) || count <= 0) {
    return null;
  }
  const { left, plotWidth } = regions(layout);
  if (plotWidth <= 0 || x < left || x > left + plotWidth) {
    return null;
  }
  const slot = plotWidth / count;
  const index = Math.floor((x - left) / slot);
  return Math.min(count - 1, Math.max(0, index));
}

/**
 * Convert a client x coordinate to the chart's viewBox x coordinate.
 *
 * The viewBox is rendered at an unknown CSS size, so both the offset and the
 * scale are read from the rendered rectangle. A zero-width rectangle has no
 * meaningful mapping and yields null.
 */
export function viewBoxX(
  clientX: number,
  rectLeft: number,
  rectWidth: number,
  viewWidth: number,
): number | null {
  if (!Number.isFinite(clientX) || !Number.isFinite(rectWidth) || rectWidth <= 0) {
    return null;
  }
  return ((clientX - rectLeft) / rectWidth) * viewWidth;
}
