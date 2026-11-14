import { describe, expect, it } from "bun:test";
import { markClassForKey, markClassLabel, MARK_CLASSES } from "./markClasses";

describe("the eight mark classes", () => {
  it("has one class for each number key", () => {
    expect(MARK_CLASSES.map((markClass) => markClass.key)).toEqual([
      "1",
      "2",
      "3",
      "4",
      "5",
      "6",
      "7",
      "8",
    ]);
  });

  it("covers entry and exit × good and bad × long and short exactly once", () => {
    const combinations = new Set(
      MARK_CLASSES.map(
        (markClass) => `${markClass.action}/${markClass.quality}/${markClass.side}`,
      ),
    );
    expect(combinations.size).toBe(8);
    for (const action of ["entry", "exit"]) {
      for (const quality of ["good", "bad"]) {
        for (const side of ["long", "short"]) {
          expect(combinations.has(`${action}/${quality}/${side}`)).toBe(true);
        }
      }
    }
  });

  it("resolves every key and rejects a key outside the table", () => {
    for (const markClass of MARK_CLASSES) {
      expect(markClassForKey(markClass.key)).toEqual(markClass);
    }
    expect(markClassForKey("0")).toBeNull();
    expect(markClassForKey("9")).toBeNull();
    expect(markClassForKey("z")).toBeNull();
  });

  it("carries a word label for every class", () => {
    for (const markClass of MARK_CLASSES) {
      expect(markClass.label.length).toBeGreaterThan(0);
      expect(markClass.label).toContain("·");
    }
  });

  it("labels an oracle's neutral quality without a keyboard class", () => {
    expect(markClassLabel({ action: "entry", quality: "neutral", side: "long" })).toBe(
      "Entry · Neutral · Long",
    );
  });
});
