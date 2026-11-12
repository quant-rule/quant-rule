import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ModelRunView } from "./types";

const { call } = vi.hoisted(() => ({ call: vi.fn() }));

vi.mock("./api", () => ({ call, isTauri: () => false }));

import RunPanel from "./RunPanel.svelte";

const model: ModelRunView = {
  run: {
    name: "fixture",
    output_root: "/mock/workspace/target/native-audit",
    config_hash: "0123456789abcdef",
    model_id: "M1.0",
    execution_id: "E1.0",
    portfolio_id: "P1.0",
    instrument: "EURUSD",
    data_version: "test:v1",
    data_source: "/mock/workspace/data/sources/state_m1.csv",
    data_timeframes: ["1m", "5m", "15m", "1h"],
    model_timeframes: ["1m", "5m", "15m", "1h"],
    execution_fill_timeframe: "5m",
    initial_cash: 10_000,
    leverage: 1,
    max_open_positions: 1,
  },
  timeframes: [
    { timeframe_secs: 60, bars: 260, events: 520, levels: 68 },
    { timeframe_secs: 300, bars: 52, events: 62, levels: 21 },
  ],
  events: 582,
};

describe("RunPanel", () => {
  beforeEach(() => {
    call.mockReset();
  });

  it("keeps the model visible when an export fails", async () => {
    call.mockResolvedValueOnce(model);
    render(RunPanel, { runPath: "/mock/workspace/config/run.json" });

    await fireEvent.click(screen.getByRole("button", { name: "Run model" }));
    expect(await screen.findByText("EURUSD")).toBeInTheDocument();

    call.mockRejectedValueOnce(
      new Error(
        "io error: /mock/workspace/target/native-audit/run-summary.json: No such file or directory",
      ),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Export summary" }));

    expect(await screen.findByText(/io error/)).toBeInTheDocument();
    expect(screen.getByText("EURUSD")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Export summary" }),
    ).toBeInTheDocument();
  });
});
