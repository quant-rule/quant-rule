/**
 * Presentation helpers for the simulation marks.
 *
 * These functions order and count the rows the server returned so the screen
 * can show the human set beside the oracle set. They do not pair marks, judge
 * agreement, or derive a rule: that comparison belongs to the service's
 * review, and the client never recomputes a server measurement.
 */

import type { ChartMark, MarkOrigin } from "./chart/types";

/** Marks per quality, side, and action. */
export interface MarkCounts {
  total: number;
  good: number;
  bad: number;
  neutral: number;
  long: number;
  short: number;
  entry: number;
  exit: number;
}

/** A stable empty count. */
export function emptyCounts(): MarkCounts {
  return {
    total: 0,
    good: 0,
    bad: 0,
    neutral: 0,
    long: 0,
    short: 0,
    entry: 0,
    exit: 0,
  };
}

/**
 * Count one origin's marks, or every mark when no origin is named.
 *
 * Counting is presentation: the table needs a total per side, and the service
 * remains the owner of the review itself.
 */
export function countMarks(
  marks: readonly ChartMark[],
  origin?: MarkOrigin,
): MarkCounts {
  const counts = emptyCounts();
  for (const mark of marks) {
    if (origin !== undefined && mark.origin !== origin) {
      continue;
    }
    counts.total += 1;
    if (mark.quality === "good") {
      counts.good += 1;
    } else if (mark.quality === "bad") {
      counts.bad += 1;
    } else {
      counts.neutral += 1;
    }
    if (mark.side === "long") {
      counts.long += 1;
    } else {
      counts.short += 1;
    }
    if (mark.action === "entry") {
      counts.entry += 1;
    } else {
      counts.exit += 1;
    }
  }
  return counts;
}

/** The marks of one window, in display order. */
export function marksForWindow(
  marks: readonly ChartMark[],
  chart: number,
): ChartMark[] {
  return sortMarks(marks.filter((mark) => mark.chart === chart));
}

/**
 * Order marks by window and bar, with the mark id as the tie-break.
 *
 * The order is fixed so the same server rows always render the same table,
 * whatever order the response carried.
 */
export function sortMarks(marks: readonly ChartMark[]): ChartMark[] {
  return [...marks].sort((left, right) => {
    if (left.chart !== right.chart) {
      return left.chart - right.chart;
    }
    if (left.bar !== right.bar) {
      return left.bar - right.bar;
    }
    return textOrder(left.mark_id, right.mark_id);
  });
}

function textOrder(left: string, right: string): number {
  if (left < right) {
    return -1;
  }
  return left > right ? 1 : 0;
}
