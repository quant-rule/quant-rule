import { describe, expect, it } from "bun:test";
import { canSubmit } from "./configs";
import type { ConfigSummary } from "./types";

function config(valid: boolean): ConfigSummary {
  return {
    config_id: "cfg_abc",
    name: "valid/run.json",
    origin: "repository",
    valid,
    message: valid ? null : "missing-data.json is unreadable",
    config_hash: valid ? "sha256:" + "a".repeat(64) : null,
  };
}

describe("canSubmit", () => {
  it("requires a valid config, a dataset, and no submission in flight", () => {
    expect(canSubmit(config(true), "sha256:" + "a".repeat(64), null)).toBe(true);
  });

  it("refuses an invalid config", () => {
    expect(canSubmit(config(false), "sha256:" + "a".repeat(64), null)).toBe(false);
  });

  it("refuses without a dataset", () => {
    expect(canSubmit(config(true), "", null)).toBe(false);
  });

  it("refuses while another submission is in flight", () => {
    expect(canSubmit(config(true), "sha256:" + "a".repeat(64), "cfg_other")).toBe(false);
  });
});
