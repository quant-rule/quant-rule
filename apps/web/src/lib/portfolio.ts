/**
 * Read the published execution and portfolio artifacts.
 *
 * The viewer parses the artifact's own columns and JSON fields. It never
 * recomputes a P&L, a metric, or a fill; a missing value stays missing and a
 * published value is rendered as stored.
 */

import type {
  PortfolioMetrics,
  PortfolioSummaryEntry,
  PortfolioTable,
  PortfolioView,
} from "./types";

const REQUIRED = [
  "execution/orders.csv",
  "execution/fills.csv",
  "execution/rejections.csv",
  "execution/summary.txt",
  "portfolio/trades.csv",
  "portfolio/risk_events.csv",
  "portfolio/metrics.json",
  "portfolio/summary.txt",
] as const;

/** Parse one comma-separated artifact with a header row. */
export function parseCsv(text: string): PortfolioTable {
  const lines = text
    .split("\n")
    .map((line) => line.replace(/\r$/, ""))
    .filter((line) => line.length > 0);
  const header = lines.shift();
  if (header === undefined) {
    throw new Error("the artifact has no header");
  }
  const columns = header.split(",");
  const rows: string[][] = [];
  for (const [index, line] of lines.entries()) {
    const values = line.split(",");
    if (values.length !== columns.length) {
      throw new Error(
        `row ${index + 2} has ${values.length} fields, not ${columns.length}`,
      );
    }
    rows.push(values);
  }
  return { columns, rows };
}

/** Parse the published metrics JSON without recomputing a value. */
export function parsePortfolioMetrics(text: string): PortfolioMetrics {
  const value = JSON.parse(text) as unknown;
  if (value === null || typeof value !== "object") {
    throw new Error("metrics.json is not an object");
  }
  const metrics = value as Record<string, unknown>;
  if (metrics.schema !== "quantrule.portfolio-metrics/v1") {
    throw new Error("metrics.json names another schema");
  }
  for (const field of [
    "initial_cash",
    "final_equity",
    "net_pnl",
    "execution_fees",
    "commission",
    "funding_paid",
    "funding_received",
    "max_drawdown",
  ]) {
    if (typeof metrics[field] !== "number") {
      throw new Error(`metrics.json has no numeric ${field}`);
    }
  }
  if (metrics.exposure !== null && typeof metrics.exposure !== "number") {
    throw new Error("metrics.json has no numeric or null exposure");
  }
  if (typeof metrics.halted !== "boolean") {
    throw new Error("metrics.json has no boolean halted");
  }
  return metrics as unknown as PortfolioMetrics;
}

/** Split a published `key: value` summary into its lines. */
export function parseSummary(text: string): PortfolioSummaryEntry[] {
  return text
    .split("\n")
    .filter((line) => line.trim() !== "")
    .map((line) => {
      const index = line.indexOf(":");
      if (index < 0) {
        return { label: line, value: "" };
      }
      return {
        label: line.slice(0, index).trim(),
        value: line.slice(index + 1).trim(),
      };
    });
}

/** Build the portfolio view from the published artifact texts. */
export function portfolioView(files: Record<string, string>): PortfolioView {
  for (const name of REQUIRED) {
    if (files[name] === undefined) {
      throw new Error(`the stage artifact has no ${name}`);
    }
  }
  return {
    orders: parseCsv(files["execution/orders.csv"] ?? ""),
    fills: parseCsv(files["execution/fills.csv"] ?? ""),
    rejections: parseCsv(files["execution/rejections.csv"] ?? ""),
    executionSummary: parseSummary(files["execution/summary.txt"] ?? ""),
    trades: parseCsv(files["portfolio/trades.csv"] ?? ""),
    riskEvents: parseCsv(files["portfolio/risk_events.csv"] ?? ""),
    metrics: parsePortfolioMetrics(files["portfolio/metrics.json"] ?? ""),
    summary: parseSummary(files["portfolio/summary.txt"] ?? ""),
  };
}
