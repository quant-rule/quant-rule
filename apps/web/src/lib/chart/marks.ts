/**
 * The session's mark list and the draft it posts.
 *
 * The server is the record; this module only builds the request shape and keeps
 * the acknowledged marks ordered. Undo is a list operation here, and the
 * component pairs it with the delete call so the visible list and the server
 * do not drift.
 */

import type { ChartMark, MarkDraft } from "./types";
import type { MarkClass } from "./markClasses";

/** The longest note the mark schema accepts. */
export const NOTE_LIMIT = 500;

/** Cap a note at the schema limit. */
export function capNote(note: string): string {
  return note.length > NOTE_LIMIT ? note.slice(0, NOTE_LIMIT) : note;
}

/** Build the request body for one class at one bar. */
export function draftForClass(
  markClass: MarkClass,
  chart: number,
  bar: number,
  note: string,
): MarkDraft {
  return {
    chart,
    bar,
    action: markClass.action,
    quality: markClass.quality,
    side: markClass.side,
    note: capNote(note),
  };
}

/** Append one acknowledged mark without mutating the current list. */
export function appendMark(
  marks: readonly ChartMark[],
  mark: ChartMark,
): ChartMark[] {
  return [...marks, mark];
}

/** Remove one acknowledged mark by id without mutating the current list. */
export function removeMarkById(
  marks: readonly ChartMark[],
  markId: string,
): ChartMark[] {
  return marks.filter((mark) => mark.mark_id !== markId);
}

/**
 * The last mark this session posted and still shows.
 *
 * A mark loaded from the server is not a session mark, so undo never deletes an
 * oracle mark or a mark another session recorded.
 */
export function lastSessionMark(
  marks: readonly ChartMark[],
  postedIds: readonly string[],
): ChartMark | null {
  for (let index = marks.length - 1; index >= 0; index -= 1) {
    const mark = marks[index];
    if (mark !== undefined && postedIds.includes(mark.mark_id)) {
      return mark;
    }
  }
  return null;
}
