import { afterEach, describe, expect, it } from "vitest";

import { recordTimes, setLastRun } from "./session.svelte";

describe("recordTimes", () => {
  afterEach(() => {
    setLastRun(null);
  });

  it("uses the completed run's own timestamps", () => {
    setLastRun({
      config: "/a/run.json",
      startedAt: 10,
      finishedAt: 20,
      model: null,
    });
    expect(recordTimes("/a/run.json", 99)).toEqual({
      startedAt: 10,
      finishedAt: 20,
    });
  });

  it("uses a null finish for an unknown run instead of a zero duration", () => {
    setLastRun({
      config: "/a/run.json",
      startedAt: 10,
      finishedAt: 20,
      model: null,
    });
    expect(recordTimes("/b/run.json", 99)).toEqual({
      startedAt: 99,
      finishedAt: null,
    });
  });
});
