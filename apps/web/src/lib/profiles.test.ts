import { describe, expect, it } from "bun:test";
import { chooseProfile } from "./profiles";
import type { ProfileSummary } from "./types";

function profile(id: string): ProfileSummary {
  return { profile_id: id, name: id, datasets: 0, runs: 0, simulations: 0 };
}

describe("chooseProfile", () => {
  it("keeps a stored profile while the server lists it", () => {
    expect(chooseProfile([profile("default"), profile("alpha")], "alpha")).toEqual({
      active: "alpha",
      unknown: false,
    });
  });

  it("keeps a stored profile the server no longer lists, and marks it unknown", () => {
    expect(chooseProfile([profile("default"), profile("alpha")], "removed")).toEqual({
      active: "removed",
      unknown: true,
    });
  });

  it("selects default on a first run", () => {
    expect(chooseProfile([profile("default"), profile("alpha")], null)).toEqual({
      active: "default",
      unknown: false,
    });
  });

  it("uses the first profile when default is absent", () => {
    expect(chooseProfile([profile("alpha"), profile("beta")], null)).toEqual({
      active: "alpha",
      unknown: false,
    });
  });

  it("keeps default on an empty server", () => {
    expect(chooseProfile([], null)).toEqual({ active: "default", unknown: false });
  });
});
