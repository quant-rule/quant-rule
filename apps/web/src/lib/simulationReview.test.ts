import { describe, expect, it } from "bun:test";
import type { ChartMark } from "./chart/types";
import { countMarks, emptyCounts, marksForWindow, sortMarks } from "./simulationReview";

function mark(
  markId: string,
  chart: number,
  bar: number,
  origin: ChartMark["origin"] = "human",
): ChartMark {
  return {
    mark_id: markId,
    chart,
    bar,
    action: "entry",
    quality: "good",
    side: "long",
    origin,
    note: "",
  };
}

describe("countMarks", () => {
  it("counts every field and filters by origin", () => {
    const marks: ChartMark[] = [
      mark("m1", 0, 1, "human"),
      { ...mark("m2", 0, 2, "human"), action: "exit", quality: "bad", side: "short" },
      { ...mark("o1", 0, 3, "oracle"), action: "exit", quality: "neutral", side: "short" },
    ];

    expect(emptyCounts()).toEqual({
      total: 0,
      good: 0,
      bad: 0,
      neutral: 0,
      long: 0,
      short: 0,
      entry: 0,
      exit: 0,
    });

    expect(countMarks(marks)).toEqual({
      total: 3,
      good: 1,
      bad: 1,
      neutral: 1,
      long: 1,
      short: 2,
      entry: 1,
      exit: 2,
    });

    expect(countMarks(marks, "human").total).toBe(2);
    expect(countMarks(marks, "oracle")).toMatchObject({
      total: 1,
      neutral: 1,
      short: 1,
      exit: 1,
    });
  });
});

describe("sortMarks", () => {
  it("orders by window, then bar, then mark id without mutating the input", () => {
    const marks = [mark("b", 1, 2), mark("c", 0, 3), mark("a", 0, 3)];
    const ordered = sortMarks(marks);
    expect(ordered.map((item) => item.mark_id)).toEqual(["a", "c", "b"]);
    expect(marks.map((item) => item.mark_id)).toEqual(["b", "c", "a"]);
  });
});

describe("marksForWindow", () => {
  it("keeps only the named window", () => {
    const marks = [mark("m1", 0, 1), mark("m2", 1, 1), mark("m3", 1, 2)];
    expect(marksForWindow(marks, 1).map((item) => item.mark_id)).toEqual([
      "m2",
      "m3",
    ]);
    expect(marksForWindow(marks, 2)).toEqual([]);
  });
});
