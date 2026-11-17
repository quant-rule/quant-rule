import { describe, expect, it } from "bun:test";

import {
  parseCsv,
  parsePortfolioMetrics,
  parseSummary,
  portfolioView,
} from "./portfolio";

const METRICS =
  '{"schema":"quantrule.portfolio-metrics/v1","initial_cash":10000.0,' +
  '"final_equity":10020.0,"net_pnl":20.0,"execution_fees":1.0,' +
  '"commission":2.0,"funding_paid":3.0,"funding_received":4.0,' +
  '"max_drawdown":0.01,"halted":false}\n';
const CSV = "fill_id,side,quantity\n0,buy,10\n1,sell,10\n";
const SUMMARY = "QuantRule portfolio replay\nfinal_equity: 10020\nhalted: false\n";

function files(): Record<string, string> {
  return {
    "execution/orders.csv": "order_id,side\n0,buy\n",
    "execution/fills.csv": CSV,
    "execution/rejections.csv": "order_id,reason\n",
    "execution/summary.txt": "costs: fee=1 slippage=2\n",
    "portfolio/trades.csv": "trade_id,side\n0,long\n",
    "portfolio/risk_events.csv": "bar,drawdown\n",
    "portfolio/metrics.json": METRICS,
    "portfolio/summary.txt": SUMMARY,
  };
}

describe("portfolio artifacts", () => {
  it("reads a published table without changing a value", () => {
    const table = parseCsv(CSV);
    expect(table.columns).toEqual(["fill_id", "side", "quantity"]);
    expect(table.rows).toEqual([
      ["0", "buy", "10"],
      ["1", "sell", "10"],
    ]);
  });

  it("refuses a row that does not match the header", () => {
    expect(() => parseCsv("a,b\n1\n")).toThrow();
  });

  it("reads the published metrics and summary", () => {
    const metrics = parsePortfolioMetrics(METRICS);
    expect(metrics.final_equity).toBe(10020);
    expect(metrics.net_pnl).toBe(20);
    expect(metrics.halted).toBe(false);
    expect(parseSummary(SUMMARY)).toEqual([
      { label: "QuantRule portfolio replay", value: "" },
      { label: "final_equity", value: "10020" },
      { label: "halted", value: "false" },
    ]);
  });

  it("builds the view from every required artifact", () => {
    const view = portfolioView(files());
    expect(view.fills.rows).toHaveLength(2);
    expect(view.metrics.initial_cash).toBe(10000);
    expect(view.executionSummary[0]?.value).toBe("fee=1 slippage=2");
  });

  it("refuses a missing required artifact", () => {
    const missing = files();
    delete missing["portfolio/metrics.json"];
    expect(() => portfolioView(missing)).toThrow();
  });
});
