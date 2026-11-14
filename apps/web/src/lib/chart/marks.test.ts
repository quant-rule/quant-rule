import { describe, expect, it } from "bun:test";
import type { ChartMark } from "./types";
import {
  appendMark,
  capNote,
  draftForClass,
  lastSessionMark,
  NOTE_LIMIT,
  removeMarkById,
} from "./marks";
import { markClassForKey } from "./markClasses";

function mark(id: string): ChartMark {
  return {
    mark_id: id,
    chart: 1,
    bar: 3,
    action: "entry",
    quality: "good",
    side: "long",
    origin: "human",
    note: "",
  };
}

describe("draftForClass", () => {
  it("builds the exact request body for a class", () => {
    const entry = markClassForKey("1");
    expect(entry).not.toBeNull();
    if (entry === null) {
      return;
    }
    expect(draftForClass(entry, 2, 17, "higher low")).toEqual({
      chart: 2,
      bar: 17,
      action: "entry",
      quality: "good",
      side: "long",
      note: "higher low",
    });
  });
});

describe("capNote", () => {
  it("keeps a note at the limit and caps one past it", () => {
    const atLimit = "a".repeat(NOTE_LIMIT);
    const pastLimit = "a".repeat(NOTE_LIMIT + 1);
    expect(capNote(atLimit)).toBe(atLimit);
    expect(capNote(pastLimit)).toHaveLength(NOTE_LIMIT);
    expect(capNote(pastLimit)).toBe(atLimit);
  });

  it("leaves an empty note empty", () => {
    expect(capNote("")).toBe("");
  });
});

describe("undo", () => {
  it("removes the last session mark and leaves the earlier ones in order", () => {
    const marks = appendMark(appendMark([], mark("m1")), mark("m2"));
    const last = lastSessionMark(marks, ["m1", "m2"]);
    expect(last?.mark_id).toBe("m2");
    expect(removeMarkById(marks, "m2").map((item) => item.mark_id)).toEqual(["m1"]);
    expect(marks.map((item) => item.mark_id)).toEqual(["m1", "m2"]);
  });

  it("is safe with no marks", () => {
    expect(removeMarkById([], "m1")).toEqual([]);
    expect(lastSessionMark([], [])).toBeNull();
  });

  it("does not offer a server-loaded mark as a session undo", () => {
    const marks = [mark("loaded"), mark("posted")];
    expect(lastSessionMark(marks, ["posted"])?.mark_id).toBe("posted");
    expect(lastSessionMark(marks, [])).toBeNull();
  });
});
