import { describe, expect, it } from "vitest";

import { call, mockMode } from "./api";
import type { WorkspaceView } from "./types";

describe("api", () => {
  it("uses the fixture backend outside Tauri", () => {
    expect(mockMode()).toBe(true);
  });

  it("opens the fixture workspace", async () => {
    const workspace = await call<WorkspaceView>("open_workspace", {
      root: "/mock/workspace",
    });
    expect(workspace.name).toBe("workspace");
    expect(workspace.datasets).toHaveLength(1);
    expect(workspace.runs).toHaveLength(1);
  });

  it("rejects a missing required argument", async () => {
    await expect(call("dataset_summary", {})).rejects.toThrow("path is required");
  });
});
