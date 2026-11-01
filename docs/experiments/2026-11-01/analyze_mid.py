#!/usr/bin/env python3
"""Analyze the EXP-2026-11-01-04 mid-timeframe matrix.

Reuses the metric helpers from `analyze.py` so the evaluation definitions stay
identical to EXP-2026-11-01-03. Uses only the Python standard library and is
deterministic.

Usage:
    python3 docs/experiments/2026-11-01/analyze_mid.py <root> \
        [--control-root <root>] [--json <path>] [--text <path>]
"""

import argparse
import json
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze as base  # noqa: E402

INSTRUMENTS = ["BTCUSD", "ETHUSD", "XAUUSD"]
YEARS = [2021, 2022, 2024]
OOS_YEARS = [2024]
GREEN_MEAN_RETURN = 0.10
GREEN_MAX_DRAWDOWN = 0.20
GREEN_MIN_TRADES = 30
GREEN_MIN_PROFIT_FACTOR = 1.2
GREEN_MAX_CONTRIBUTION = 0.60


def cell_name(instrument, year, variant="mid"):
    return f"{instrument.lower()}-{variant}-{year}"


def period_for(root, instrument, years, variant="mid"):
    dirs = [root / cell_name(instrument, year, variant) for year in years]
    return base.period_metrics(dirs, years)


def evaluate(mid):
    instruments = [mid[instrument]["oos"] for instrument in INSTRUMENTS]
    returns = [item["return"] for item in instruments]
    positive = [item for item in instruments if item["return"] > 0.0]
    total_net = sum(item["net_pnl"] for item in instruments)
    shares = [item["net_pnl"] / total_net for item in instruments] if total_net > 0 else []
    pooled = [value for item in instruments for value in item["trade_net_pnl"]]
    pooled_ci = base.confidence_interval(pooled)
    criteria = {
        "two_of_three_positive": len(positive) >= 2,
        "mean_return_at_least_10pct": statistics.fmean(returns) >= GREEN_MEAN_RETURN,
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


def strip_trade_values(metrics):
    return {key: value for key, value in metrics.items() if key != "trade_net_pnl"}


def text_report(results):
    lines = [
        "# EXP-2026-11-01-04 results",
        "",
        "Evaluation-year metrics are rebased on the first equity point on or after",
        "1 January. Full-run metrics are in the JSON output.",
        "",
        "## Treatment cell metrics (evaluation year)",
        "",
        "| Cell | Return | Max DD | Trades | PF | Win rate | Net P&L | Cost drag | Halted |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for instrument in INSTRUMENTS:
        for year in YEARS:
            cell = cell_name(instrument, year)
            metrics = results["cells"].get(cell, {}).get("year")
            lines.append(
                "| {cell} | {ret} | {dd} | {trades} | {pf} | {wr} | {net} | {cost} | {halt} |".format(
                    cell=cell,
                    ret=base.fmt(metrics["return"]) if metrics else "—",
                    dd=base.fmt(metrics["max_drawdown"]) if metrics else "—",
                    trades=metrics["trades"] if metrics else "—",
                    pf=base.fmt(metrics["profit_factor"]) if metrics else "—",
                    wr=base.fmt(metrics["win_rate"]) if metrics else "—",
                    net=base.fmt(metrics["net_pnl"], 2) if metrics else "—",
                    cost=base.fmt(metrics["cost_drag"], 2) if metrics else "—",
                    halt=base.fmt(metrics["halted"]) if metrics else "—",
                )
            )

    for title, key, label in [
        ("In-sample 2021–2022", "is", "2y"),
        ("Out-of-sample 2024", "oos", "1y"),
    ]:
        lines.extend(["", f"## {title}", ""])
        lines.append(
            f"| Variant | Instrument | {label} return | Max DD | Trades | PF | CI lower | Halted |"
        )
        lines.append("|---|---|---|---|---|---|---|---|")
        for variant, table in [("mid", results["mid"]), ("control", results.get("control", {}))]:
            for instrument in INSTRUMENTS:
                metrics = table.get(instrument, {}).get(key)
                if not metrics:
                    lines.append(f"| {variant} | {instrument} | — | — | — | — | — | — |")
                    continue
                lines.append(
                    "| {variant} | {instrument} | {ret} | {dd} | {trades} | {pf} | {ci} | {halt} |".format(
                        variant=variant,
                        instrument=instrument,
                        ret=base.fmt(metrics["return"]),
                        dd=base.fmt(metrics["max_drawdown"]),
                        trades=metrics["trades"],
                        pf=base.fmt(metrics["profit_factor"]),
                        ci=base.fmt(metrics["confidence_interval"]["lower"]),
                        halt=base.fmt(metrics["halted"]),
                    )
                )

    lines.extend(["", "## Green check (2024 out-of-sample only)", ""])
    lines.append(
        "| 2/3 positive | Mean >= 10% | DD <= 20% | Trades >= 30 | PF >= 1.2 | CI low > 0 | Contribution <= 60% | Green |"
    )
    lines.append("|---|---|---|---|---|---|---|---|")
    check = results["check"]
    criteria = check["criteria"]
    lines.append(
        "| {a} | {b} | {c} | {d} | {e} | {f} | {g} | {green} |".format(
            a=base.fmt(criteria.get("two_of_three_positive")),
            b=base.fmt(criteria.get("mean_return_at_least_10pct")),
            c=base.fmt(criteria.get("drawdown_within_20pct")),
            d=base.fmt(criteria.get("trades_at_least_30")),
            e=base.fmt(criteria.get("positive_profit_factor_at_least_1_2")),
            f=base.fmt(criteria.get("pooled_ci_lower_above_zero")),
            g=base.fmt(criteria.get("max_contribution_at_most_60pct")),
            green=base.fmt(check["green"]),
        )
    )
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--control-root", type=Path)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--text", type=Path)
    args = parser.parse_args()

    if not args.root.is_dir():
        print(f"error: not a directory: {args.root}", file=sys.stderr)
        return 2

    cells = {}
    for instrument in INSTRUMENTS:
        for year in YEARS:
            cell = cell_name(instrument, year)
            cell_dir = args.root / cell
            if not (cell_dir / "portfolio" / "report.json").is_file():
                print(f"error: missing portfolio report for {cell}", file=sys.stderr)
                return 1
            cells[cell] = {
                "full": base.full_metrics(cell_dir),
                "year": base.year_metrics(cell_dir, year),
            }

    mid = {instrument: {} for instrument in INSTRUMENTS}
    for instrument in INSTRUMENTS:
        mid[instrument]["is"] = period_for(args.root, instrument, [2021, 2022])
        mid[instrument]["oos"] = period_for(args.root, instrument, OOS_YEARS)
        if mid[instrument]["is"] is None or mid[instrument]["oos"] is None:
            print(f"error: incomplete metrics for {instrument}", file=sys.stderr)
            return 1

    control = {}
    if args.control_root and args.control_root.is_dir():
        for instrument in INSTRUMENTS:
            is_metrics = period_for(args.control_root, instrument, [2021, 2022], "control")
            oos_metrics = period_for(args.control_root, instrument, OOS_YEARS, "control")
            if is_metrics is not None and oos_metrics is not None:
                control[instrument] = {"is": is_metrics, "oos": oos_metrics}

    check = evaluate(mid)
    results = {
        "experiment": "EXP-2026-11-01-04",
        "root": str(args.root),
        "control_root": str(args.control_root) if args.control_root else None,
        "green_criteria": {
            "mean_return": GREEN_MEAN_RETURN,
            "max_drawdown": GREEN_MAX_DRAWDOWN,
            "min_trades": GREEN_MIN_TRADES,
            "min_profit_factor": GREEN_MIN_PROFIT_FACTOR,
            "max_contribution": GREEN_MAX_CONTRIBUTION,
        },
        "cells": cells,
        "mid": {
            instrument: {
                period: strip_trade_values(metrics)
                for period, metrics in periods.items()
            }
            for instrument, periods in mid.items()
        },
        "control": {
            instrument: {
                period: strip_trade_values(metrics)
                for period, metrics in periods.items()
            }
            for instrument, periods in control.items()
        },
        "check": check,
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
