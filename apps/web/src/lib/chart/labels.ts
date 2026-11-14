/**
 * Chart-only code-to-word mapping.
 *
 * The model publishes numeric codes; the generated table is the one place a
 * code becomes a word. An unknown code is shown as `unknown(<code>)` and never
 * guessed. The chart never edits or regenerates the table.
 */

import {
  DIVERGENCE_SUBTYPES,
  EVENTS,
  EVENT_TYPES,
  INDICATORS,
  LEVEL_SUBTYPES,
  SCALES,
  SIGNAL_SUBTYPES,
  SOURCES,
} from "../labels.generated";
import type { ChartEvent } from "./types";

function unknown(code: number): string {
  return `unknown(${code})`;
}

/** The registry event name for a code, or its unknown form. */
export function eventName(code: number): string {
  return EVENTS[code] ?? unknown(code);
}

/** The event type word for a code, or its unknown form. */
export function eventType(eventTypeCode: number): string {
  return EVENT_TYPES[eventTypeCode] ?? unknown(eventTypeCode);
}

/** The subtype word for one event, using the table that belongs to its type. */
export function eventSubtype(event: ChartEvent): string {
  const code = event.event_subtype_code;
  switch (event.event_type) {
    case 1:
      return LEVEL_SUBTYPES[code] ?? unknown(code);
    case 2:
      return DIVERGENCE_SUBTYPES[code] ?? unknown(code);
    case 3:
      return SIGNAL_SUBTYPES[code] ?? unknown(code);
    default:
      return unknown(code);
  }
}

/** The source reading word for a code, or its unknown form. */
export function sourceName(code: number): string {
  return SOURCES[code] ?? unknown(code);
}

/** One line naming the event type, subtype, and source. */
export function eventDetail(event: ChartEvent): string {
  return `${eventType(event.event_type)} · ${eventSubtype(event)} · ${sourceName(event.source_code)}`;
}

/** The indicator name for the one panel, or its unknown form. */
export function indicatorName(code: number): string {
  return INDICATORS[code] ?? unknown(code);
}

/** The pivot scale word for a confirmed level, or its unknown form. */
export function scaleName(code: number): string {
  return SCALES[code] ?? unknown(code);
}
