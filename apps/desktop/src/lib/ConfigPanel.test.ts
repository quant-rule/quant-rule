import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import ConfigPanel from "./ConfigPanel.svelte";

describe("ConfigPanel", () => {
  it("validates a run and offers it to the run view", async () => {
    let selected = "";
    render(ConfigPanel, {
      runs: ["/mock/workspace/config/examples/btcusd-2024/run.json"],
      onSelect: (path: string) => {
        selected = path;
      },
    });

    const run = screen.getByText(
      "/mock/workspace/config/examples/btcusd-2024/run.json",
    );
    await fireEvent.click(run);
    expect(await screen.findByText("BTCUSD")).toBeInTheDocument();
    expect(screen.getAllByText("1m, 5m, 15m, 1h")).toHaveLength(2);

    await fireEvent.click(screen.getByText("Use in Run"));
    expect(selected).toBe("/mock/workspace/config/examples/btcusd-2024/run.json");
  });
});
