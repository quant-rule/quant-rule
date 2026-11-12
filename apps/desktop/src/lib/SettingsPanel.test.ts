import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { pickDirectory, pickSaveFile } = vi.hoisted(() => ({
  pickDirectory: vi.fn(),
  pickSaveFile: vi.fn(),
}));

vi.mock("./api", () => ({ isTauri: () => true }));
vi.mock("./paths", () => ({ pickDirectory, pickSaveFile }));

import SettingsPanel from "./SettingsPanel.svelte";

describe("SettingsPanel", () => {
  beforeEach(() => {
    pickDirectory.mockReset();
    pickSaveFile.mockReset();
  });

  it("fills the path fields from the native pickers", async () => {
    pickDirectory.mockResolvedValue("/mock/workspace");
    pickSaveFile.mockResolvedValue("/mock/workspace/research.db");
    render(SettingsPanel, { workspaceRoot: "", memoryPath: "" });

    const [workspaceBrowse, memoryBrowse] = screen.getAllByRole("button", {
      name: "Browse…",
    });
    await fireEvent.click(workspaceBrowse);
    expect(pickDirectory).toHaveBeenCalledWith("Choose a workspace root");
    expect(screen.getByPlaceholderText("/home/you/quant-rule-workspace")).toHaveValue(
      "/mock/workspace",
    );

    await fireEvent.click(memoryBrowse);
    expect(pickSaveFile).toHaveBeenCalledWith(
      "Choose the research memory database",
      "/mock/workspace/research.db",
      [{ name: "SQLite database", extensions: ["db", "sqlite"] }],
    );
    expect(
      screen.getByPlaceholderText("/home/you/quant-rule-workspace/research.db"),
    ).toHaveValue("/mock/workspace/research.db");
  });
});
