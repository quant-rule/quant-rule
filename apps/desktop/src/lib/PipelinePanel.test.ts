import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { call } = vi.hoisted(() => ({ call: vi.fn() }));

vi.mock("./api", () => ({ call, isTauri: () => false }));

import PipelinePanel from "./PipelinePanel.svelte";

describe("PipelinePanel", () => {
  beforeEach(() => {
    call.mockReset();
  });

  it("runs the selected stage through the shared command", async () => {
    call.mockResolvedValue({ root: "/mock/model", artifacts: ["readings.csv"] });
    render(PipelinePanel, { runPath: "/mock/workspace/config/run.json" });

    await fireEvent.click(screen.getByRole("button", { name: "Run stage" }));

    expect(call).toHaveBeenCalledWith("run_model_report", {
      config: "/mock/workspace/config/run.json",
    });
    expect(await screen.findByText(/readings.csv/)).toBeInTheDocument();
  });
});
