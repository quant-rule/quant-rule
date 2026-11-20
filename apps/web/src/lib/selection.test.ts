import { describe, expect, it } from "bun:test";

import { selectionIsCurrent } from "./selection";

describe("selectionIsCurrent", () => {
  it("accepts the current generation and the selected id", () => {
    expect(selectionIsCurrent(2, "run-b", 2, "run-b")).toBe(true);
  });

  it("rejects a late response for another selection", () => {
    expect(selectionIsCurrent(2, "run-a", 2, "run-b")).toBe(false);
  });

  it("rejects a response from a previous profile generation", () => {
    expect(selectionIsCurrent(1, "run-b", 2, "run-b")).toBe(false);
  });
});
