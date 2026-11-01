#!/usr/bin/env python3
"""Analyze the EXP-2026-11-01-03 matrix.

Reads each cell's portfolio report and CSVs, computes full-run, evaluation-year,
in-sample, and out-of-sample metrics, and evaluates the pre-registered green
criteria. Uses only the Python standard library and is deterministic.

Usage:
    python3 docs/experiments/2026-11-01/analyze.py <root> [--json <path>] [--text <path>]
"""

import argparse
import csv
import datetime as dt
import json
import math
import statistics
import sys
from pathlib import Path

INSTRUMENTS = ["BTCUSD", "ETHUSD", "XAUUSD"]
VARIANTS = ["control", "t2", "t3"]
YEARS = [2021, 2022, 2023, 2024]
OOS_YEARS = [2023, 2024]
GREEN_MEAN_RETURN = 0.21
GREEN_MAX_DRAWDOWN = 0.20
GREEN_MIN_TRADES = 30
GREEN_MIN_PROFIT_FACTOR = 1.2
GREEN_MAX_CONTRIBUTION = 0.60
Z_95 = 1.959963984540054


def money(value):
    return float(value)


def read_csv(path):
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def read_report(path):
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def date_of_timestamp(value):
    return dt.datetime.fromtimestamp(int(value), dt.timezone.utc).date()


def profit_factor(wins, losses):
    if losses == 0.0:
        return None
    return wins / losses


def trade_metrics(trades):
    net = [money(row["net_pnl"]) for row in trades]
    gross = [money(row["gross_pnl"]) for row in trades]
    wins = [value for value in net if value > 0.0]
    losses = [value for value in net if value < 0.0]
    return {
        "trades": len(trades),
        "net_pnl": sum(net),
        "gross_pnl": sum(gross),
        "commission": sum(money(row["commission"]) for row in trades),
        "execution_fees": sum(money(row["execution_fees"]) for row in trades),
        "funding": sum(money(row["funding"]) for row in trades),
        "wins": len(wins),
        "losses": len(losses),
        "win_rate": (len(wins) / len(trades)) if trades else None,
        "profit_factor": profit_factor(sum(wins), abs(sum(losses))),
        "average_trade": statistics.fmean(net) if net else None,
        "cost_drag": sum(
            money(row["commission"]) + money(row["execution_fees"]) + money(row["funding"])
            for row in trades
        ),
    }


def equity_metrics(points):
    if not points:
        return {
            "first_equity": None,
            "last_equity": None,
            "return": None,
            "max_drawdown": None,
            "first_date": None,
            "last_date": None,
        }
    values = [money(row["equity"]) for row in points]
    first = values[0]
    last = values[-1]
    peak = values[0]
    max_drawdown = 0.0
    for value in values:
        if value > peak:
            peak = value
        drawdown = (peak - value) / peak if peak else 0.0
        max_drawdown = max(max_drawdown, drawdown)
    return {
        "first_equity": first,
        "last_equity": last,
        "return": (last / first - 1.0) if first else None,
        "max_drawdown": max_drawdown,
        "first_date": points[0]["date"],
        "last_date": points[-1]["date"],
    }


def confidence_interval(values):
    if len(values) < 2:
        return {"n": len(values), "mean": None, "lower": None, "upper": None}
    mean = statistics.fmean(values)
    deviation = statistics.stdev(values)
    standard_error = deviation / math.sqrt(len(values))
    return {
        "n": len(values),
        "mean": mean,
        "lower": mean - Z_95 * standard_error,
        "upper": mean + Z_95 * standard_error,
    }


def year_metrics(cell_dir, year):
    trades = read_csv(cell_dir / "portfolio" / "trades.csv")
    equity = read_csv(cell_dir / "portfolio" / "equity.csv")
    report = read_report(cell_dir / "portfolio" / "report.json")
    start = dt.date(year, 1, 1)
    end = dt.date(year, 12, 31)

    points = [
        row
        for row in equity
        if start <= dt.date.fromisoformat(row["date"]) <= end
    ]
    selected_trades = [
        row
        for row in trades
        if start <= date_of_timestamp(row["entry_time"]) <= end
    ]
    risk_events = report.get("records", {}).get("risk_events", [])
    halted = any(start <= date_of_timestamp(event["time"]) <= end for event in risk_events)
    metrics = {**trade_metrics(selected_trades), **equity_metrics(points)}
    metrics["halted"] = halted
    return metrics


def full_metrics(cell_dir):
    report = read_report(cell_dir / "portfolio" / "report.json")
    summary = report["summary"]
    return {
        "return": summary.get("total_return"),
        "max_drawdown": summary.get("max_drawdown"),
        "trades": summary.get("trades"),
        "profit_factor": summary.get("profit_factor"),
        "win_rate": summary.get("win_rate"),
        "average_trade": summary.get("average_trade"),
        "exposure": summary.get("exposure"),
        "net_pnl": summary.get("net_pnl"),
        "gross_pnl": summary.get("gross_pnl"),
        "cost_drag": (
            summary.get("commission", 0.0)
            + summary.get("execution_fees", 0.0)
            + summary.get("funding_paid", 0.0)
            - summary.get("funding_received", 0.0)
        ),
        "halted": summary.get("halted"),
        "data_version": report.get("data_version"),
        "config_hash": report.get("config", {}).get("config_hash"),
    }


def combined_equity(cell_dirs, years):
    points = []
    level = 1.0
    for index, cell_dir in enumerate(cell_dirs):
        year = years[index]
        start = dt.date(year, 1, 1)
        end = dt.date(year, 12, 31)
        equity = [
            row
            for row in read_csv(cell_dir / "portfolio" / "equity.csv")
            if start <= dt.date.fromisoformat(row["date"]) <= end
        ]
        if not equity:
            return None
        first = money(equity[0]["equity"])
        for row in equity:
            if first:
                points.append(level * money(row["equity"]) / first)
        if first:
            level *= money(equity[-1]["equity"]) / first
    if not points:
        return None
    peak = points[0]
    max_drawdown = 0.0
    for value in points:
        peak = max(peak, value)
        max_drawdown = max(max_drawdown, (peak - value) / peak if peak else 0.0)
    return {
        "return": points[-1] / points[0] - 1.0 if points[0] else None,
        "max_drawdown": max_drawdown,
    }


def period_metrics(cell_dirs, years):
    year_results = [year_metrics(cell_dir, year) for cell_dir, year in zip(cell_dirs, years)]
    if any(metrics["return"] is None for metrics in year_results):
        return None
    net_values = []
    wins_sum = 0.0
    losses_sum = 0.0
    for cell_dir, year in zip(cell_dirs, years):
        start = dt.date(year, 1, 1)
        end = dt.date(year, 12, 31)
        for row in read_csv(cell_dir / "portfolio" / "trades.csv"):
            if not (start <= date_of_timestamp(row["entry_time"]) <= end):
                continue
            value = money(row["net_pnl"])
            net_values.append(value)
            if value > 0.0:
                wins_sum += value
            elif value < 0.0:
                losses_sum += -value
    combined = combined_equity(cell_dirs, years)
    compound = 1.0
    for metrics in year_results:
        compound *= 1.0 + metrics["return"]
    trades = sum(metrics["trades"] for metrics in year_results)
    wins = sum(metrics["wins"] for metrics in year_results)
    losses = sum(metrics["losses"] for metrics in year_results)
    return {
        "return": compound - 1.0,
        "max_drawdown": combined["max_drawdown"] if combined else None,
        "trades": trades,
        "wins": wins,
        "losses": losses,
        "win_rate": (wins / trades) if trades else None,
        "profit_factor": profit_factor(wins_sum, losses_sum),
        "net_pnl": sum(net_values),
        "average_trade": statistics.fmean(net_values) if net_values else None,
        "cost_drag": sum(metrics["cost_drag"] for metrics in year_results),
        "halted": any(metrics["halted"] for metrics in year_results),
        "confidence_interval": confidence_interval(net_values),
        "trade_net_pnl": net_values,
        "years": {str(year): year_results[index] for index, year in enumerate(years)},
    }


def evaluate_variant(results):
    instruments = [
        results[instrument]["oos"] for instrument in INSTRUMENTS if results.get(instrument)
    ]
    if len(instruments) != len(INSTRUMENTS):
        return {"green": False, "criteria": {"complete": False}}
    returns = [item["return"] for item in instruments]
    positive = [item for item in instruments if item["return"] > 0.0]
    total_net = sum(item["net_pnl"] for item in instruments)
    shares = [item["net_pnl"] / total_net for item in instruments] if total_net > 0 else []
    pooled = [value for item in instruments for value in item["trade_net_pnl"]]
    pooled_ci = confidence_interval(pooled)
    criteria = {
        "two_of_three_positive": len(positive) >= 2,
        "mean_return_at_least_21pct": statistics.fmean(returns) >= GREEN_MEAN_RETURN,
        "drawdown_within_20pct": all(
            item["max_drawdown"] is not None
            and item["max_drawdown"] <= GREEN_MAX_DRAWDOWN
            and not item["halted"]
            for item in instruments
        ),
        "trades_at_least_30": all(item["trades"] >= GREEN_MIN_TRADES for item in instruments),
        "positive_profit_factor_at_least_1_2": all(
            item["profit_factor"] is None or item["profit_factor"] >= GREEN_MIN_PROFIT_FACTOR
            for item in positive
        ),
        "pooled_ci_lower_above_zero": pooled_ci["lower"] is not None and pooled_ci["lower"] > 0.0,
        "max_contribution_at_most_60pct": bool(shares) and max(shares) <= GREEN_MAX_CONTRIBUTION,
    }
    return {
        "green": all(criteria.values()),
        "criteria": criteria,
        "mean_return": statistics.fmean(returns),
        "positive_count": len(positive),
        "total_net_pnl": total_net,
        "pooled_expectancy": pooled_ci,
        "max_contribution": max(shares) if shares else None,
    }


def fmt(value, digits=4):
    if value is None:
        return "—"
    if isinstance(value, bool):
        return "yes" if value else "no"
    if isinstance(value, float):
        return f"{value:.{digits}f}"
    return str(value)


def text_report(results):
    lines = [
        "# EXP-2026-11-01-03 results",
        "",
        "Evaluation-year metrics are rebased on the first equity point on or after",
        "1 January. Full-run metrics are in the JSON output.",
        "",
        "## Cell metrics (evaluation year)",
        "",
    ]
    lines.append("| Cell | Return | Max DD | Trades | PF | Win rate | Net P&L | Cost drag | Halted |")
    lines.append("|---|---|---|---|---|---|---|---|---|")
    for instrument in INSTRUMENTS:
        for variant in VARIANTS:
            for year in YEARS:
                cell = f"{instrument.lower()}-{variant}-{year}"
                metrics = results["cells"].get(cell, {}).get("year")
                if not metrics:
                    lines.append(f"| {cell} | — | — | — | — | — | — | — | — |")
                    continue
                lines.append(
                    "| {cell} | {ret} | {dd} | {trades} | {pf} | {wr} | {net} | {cost} | {halt} |".format(
                        cell=cell,
                        ret=fmt(metrics["return"]),
                        dd=fmt(metrics["max_drawdown"]),
                        trades=metrics["trades"],
                        pf=fmt(metrics["profit_factor"]),
                        wr=fmt(metrics["win_rate"]),
                        net=fmt(metrics["net_pnl"], 2),
                        cost=fmt(metrics["cost_drag"], 2),
                        halt=fmt(metrics["halted"]),
                    )
                )

    for title, key, label in [
        ("In-sample 2021–2022", "is", "2y"),
        ("Out-of-sample 2023–2024", "oos", "2y"),
    ]:
        lines.extend(["", f"## {title}", ""])
        lines.append(
            f"| Variant | Instrument | {label} return | Max DD | Trades | PF | CI lower | Halted |"
        )
        lines.append("|---|---|---|---|---|---|---|---|")
        for variant in VARIANTS:
            for instrument in INSTRUMENTS:
                metrics = results["variants"][variant].get(instrument, {}).get(key)
                if not metrics:
                    lines.append(f"| {variant} | {instrument} | — | — | — | — | — | — |")
                    continue
                lines.append(
                    "| {variant} | {instrument} | {ret} | {dd} | {trades} | {pf} | {ci} | {halt} |".format(
                        variant=variant,
                        instrument=instrument,
                        ret=fmt(metrics["return"]),
                        dd=fmt(metrics["max_drawdown"]),
                        trades=metrics["trades"],
                        pf=fmt(metrics["profit_factor"]),
                        ci=fmt(metrics["confidence_interval"]["lower"]),
                        halt=fmt(metrics["halted"]),
                    )
                )

    lines.extend(["", "## Green check (out-of-sample only)", ""])
    lines.append(
        "| Variant | 2/3 positive | Mean >= 21% | DD <= 20% | Trades >= 30 | PF >= 1.2 | CI low > 0 | Contribution <= 60% | Green |"
    )
    lines.append("|---|---|---|---|---|---|---|---|---|")
    for variant in VARIANTS:
        check = results["checks"][variant]
        criteria = check["criteria"]
        lines.append(
            "| {variant} | {a} | {b} | {c} | {d} | {e} | {f} | {g} | {green} |".format(
                variant=variant,
                a=fmt(criteria.get("two_of_three_positive")),
                b=fmt(criteria.get("mean_return_at_least_21pct")),
                c=fmt(criteria.get("drawdown_within_20pct")),
                d=fmt(criteria.get("trades_at_least_30")),
                e=fmt(criteria.get("positive_profit_factor_at_least_1_2")),
                f=fmt(criteria.get("pooled_ci_lower_above_zero")),
                g=fmt(criteria.get("max_contribution_at_most_60pct")),
                green=fmt(check["green"]),
            )
        )
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--text", type=Path)
    args = parser.parse_args()

    if not args.root.is_dir():
        print(f"error: not a directory: {args.root}", file=sys.stderr)
        return 2

    cells = {}
    for instrument in INSTRUMENTS:
        for variant in VARIANTS:
            for year in YEARS:
                cell = f"{instrument.lower()}-{variant}-{year}"
                cell_dir = args.root / cell
                if not (cell_dir / "portfolio" / "report.json").is_file():
                    print(f"error: missing portfolio report for {cell}", file=sys.stderr)
                    return 1
                cells[cell] = {
                    "full": full_metrics(cell_dir),
                    "year": year_metrics(cell_dir, year),
                }

    variants = {variant: {} for variant in VARIANTS}
    for variant in VARIANTS:
        for instrument in INSTRUMENTS:
            is_dirs = [
                args.root / f"{instrument.lower()}-{variant}-{year}" for year in [2021, 2022]
            ]
            oos_dirs = [
                args.root / f"{instrument.lower()}-{variant}-{year}" for year in OOS_YEARS
            ]
            variants[variant][instrument] = {
                "is": period_metrics(is_dirs, [2021, 2022]),
                "oos": period_metrics(oos_dirs, OOS_YEARS),
            }

    checks = {variant: evaluate_variant(variants[variant]) for variant in VARIANTS}
    output_variants = {
        variant: {
            instrument: {
                period: {
                    key: value
                    for key, value in metrics[period].items()
                    if key != "trade_net_pnl"
                }
                for period in ["is", "oos"]
            }
            for instrument, metrics in variants[variant].items()
        }
        for variant in VARIANTS
    }
    results = {
        "experiment": "EXP-2026-11-01-03",
        "root": str(args.root),
        "green_criteria": {
            "mean_return": GREEN_MEAN_RETURN,
            "max_drawdown": GREEN_MAX_DRAWDOWN,
            "min_trades": GREEN_MIN_TRADES,
            "min_profit_factor": GREEN_MIN_PROFIT_FACTOR,
            "max_contribution": GREEN_MAX_CONTRIBUTION,
        },
        "cells": cells,
        "variants": output_variants,
        "checks": checks,
    }

    text = text_report(results)
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.text:
        args.text.parent.mkdir(parents=True, exist_ok=True)
        args.text.write_text(text, encoding="utf-8")
    if not args.json and not args.text:
        print(json.dumps(results, indent=2, sort_keys=True))
    else:
        print(text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
