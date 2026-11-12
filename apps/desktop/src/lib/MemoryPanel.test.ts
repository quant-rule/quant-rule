import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { call } = vi.hoisted(() => ({ call: vi.fn() }));
const { pickOpenFile, pickSaveFile } = vi.hoisted(() => ({
  pickOpenFile: vi.fn(),
  pickSaveFile: vi.fn(),
}));

vi.mock("./api", () => ({ call, isTauri: () => true }));
vi.mock("./paths", () => ({ pickOpenFile, pickSaveFile }));

import MemoryPanel from "./MemoryPanel.svelte";

describe("MemoryPanel", () => {
  beforeEach(() => {
    call.mockReset();
    pickOpenFile.mockReset();
    pickSaveFile.mockReset();
  });

  afterEach(cleanup);

  it("fills the export path from the native save picker", async () => {
    pickSaveFile.mockResolvedValue("/mock/workspace/memory-export.json");
    render(MemoryPanel, {
      memoryPath: "/mock/workspace/research.db",
      runPath: "/mock/workspace/config/run.json",
    });

    const [exportBrowse] = screen.getAllByRole("button", { name: "Browse…" });
    await fireEvent.click(exportBrowse);

    expect(pickSaveFile).toHaveBeenCalledWith(
      "Export research memory",
      "/mock/workspace/memory-export.json",
      [{ name: "JSON", extensions: ["json"] }],
    );
    expect(screen.getByLabelText("Export path")).toHaveValue(
      "/mock/workspace/memory-export.json",
    );
  });

  it("fills the import path from the native open picker", async () => {
    pickOpenFile.mockResolvedValue("/mock/workspace/memory-export.json");
    render(MemoryPanel, {
      memoryPath: "/mock/workspace/research.db",
      runPath: "/mock/workspace/config/run.json",
    });

    const [, importBrowse] = screen.getAllByRole("button", { name: "Browse…" });
    await fireEvent.click(importBrowse);

    expect(pickOpenFile).toHaveBeenCalledWith("Import research memory", [
      { name: "JSON", extensions: ["json"] },
    ]);
    expect(screen.getByLabelText("Import path")).toHaveValue(
      "/mock/workspace/memory-export.json",
    );
  });
});
