#!/usr/bin/env python3
"""Aggregate a v9 compression-strategy matrix.

Each cell is one instrument-year backtest bundle. The script reads the
portfolio summary and trades, compounds returns per instrument, and applies the
pre-registered green criteria. It computes no strategy decision and no model
value.

Usage:
    python3 tools/improvement/strategy_v9_evaluate.py \
        --matrix config/compression/strategy-v9/dev/matrix.json \
        --out data/strategy-v9/dev/evidence
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True, help="strategy matrix JSON")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--max-drawdown", type=float, default=0.10)
    parser.add_argument("--min-trades", type=int, default=30)
    parser.add_argument("--min-profit-factor", type=float, default=1.0)
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def read_trades(path: Path) -> tuple[float, float, int, int]:
    gross_profit = 0.0
    gross_loss = 0.0
    wins = 0
    losses = 0
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                net = float(row["net_pnl"])
            except (KeyError, TypeError, ValueError):
                continue
            if net > 0.0:
                gross_profit += net
                wins += 1
            elif net < 0.0:
                gross_loss += -net
                losses += 1
    return gross_profit, gross_loss, wins, losses


def cell_metrics(run: Path) -> dict[str, Any]:
    report = read_json(run / "portfolio" / "report.json")
    summary = report.get("summary", {})
    trades_path = run / "portfolio" / "trades.csv"
    gross_profit, gross_loss, wins, losses = read_trades(trades_path)
    profit_factor = (
        gross_profit / gross_loss
        if gross_loss > 0.0
        else (math.inf if gross_profit > 0.0 else None)
    )
    return {
        "return": summary.get("total_return"),
        "max_drawdown": summary.get("max_drawdown"),
        "trades": summary.get("trades"),
        "halted": summary.get("halted"),
        "win_rate": summary.get("win_rate"),
        "profit_factor": profit_factor,
        "gross_profit": gross_profit,
        "gross_loss": gross_loss,
        "wins": wins,
        "losses": losses,
        "final_equity": summary.get("final_equity"),
        "commission": summary.get("commission"),
        "execution_fees": summary.get("execution_fees"),
    }


def evaluate(cells: list[dict[str, Any]], args: argparse.Namespace) -> dict[str, Any]:
    by_instrument: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for cell in cells:
        run = Path(cell["run"]).expanduser()
        metrics = cell_metrics(run)
        by_instrument[str(cell["instrument"])].append(
            {"fold": int(cell["fold"]), **metrics}
        )
    instruments = {}
    all_return = 0.0
    worst_drawdown = 0.0
    total_trades = 0
    total_profit = 0.0
    total_loss = 0.0
    positive = 0
    for instrument in sorted(by_instrument):
        folds = sorted(by_instrument[instrument], key=lambda value: value["fold"])
        compounded = 1.0
        drawdown = 0.0
        trades = 0
        profit = 0.0
        loss = 0.0
        win_count = 0
        loss_count = 0
        halted = 0
        for fold in folds:
            value = fold["return"]
            if value is not None:
                compounded *= 1.0 + value
            drawdown = max(drawdown, fold["max_drawdown"] or 0.0)
            trades += int(fold["trades"] or 0)
            profit += fold["gross_profit"]
            loss += fold["gross_loss"]
            win_count += fold["wins"]
            loss_count += fold["losses"]
            halted += int(bool(fold["halted"]))
        instrument_return = compounded - 1.0
        profit_factor = (
            profit / loss if loss > 0.0 else (math.inf if profit > 0.0 else None)
        )
        instruments[instrument] = {
            "return": instrument_return,
            "max_drawdown": drawdown,
            "trades": trades,
            "profit_factor": profit_factor,
            "win_rate": (
                win_count / (win_count + loss_count)
                if win_count + loss_count > 0
                else None
            ),
            "halted_folds": halted,
            "folds": folds,
        }
        all_return += instrument_return
        worst_drawdown = max(worst_drawdown, drawdown)
        total_trades += trades
        total_profit += profit
        total_loss += loss
        if instrument_return > 0.0:
            positive += 1
    average_return = all_return / len(instruments) if instruments else None
    pooled_profit_factor = (
        total_profit / total_loss
        if total_loss > 0.0
        else (math.inf if total_profit > 0.0 else None)
    )
    criteria = {
        "return": average_return is not None and average_return > 0.0,
        "max_drawdown": worst_drawdown <= args.max_drawdown,
        "trades": total_trades >= args.min_trades,
        "profit_factor": pooled_profit_factor is not None
        and pooled_profit_factor > args.min_profit_factor,
        "positive_instruments": positive >= 2,
    }
    passed = all(criteria.values())
    return {
        "schema": "quantrule.strategy-v9-evaluation/v1",
        "instruments": instruments,
        "pooled": {
            "average_instrument_return": average_return,
            "worst_instrument_drawdown": worst_drawdown,
            "trades": total_trades,
            "profit_factor": pooled_profit_factor,
            "positive_instruments": positive,
        },
        "criteria": criteria,
        "passed": passed,
    }


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# Strategy v9 — 1m Compression Breakout",
        "",
        f"- Decision: **{'pass' if document['passed'] else 'no edge'}**",
        "",
        "| Instrument | Return | Max DD | Trades | PF | Win | Halted folds |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for instrument in sorted(document["instruments"]):
        value = document["instruments"][instrument]
        lines.append(
            "| {name} | {ret} | {dd} | {trades} | {pf} | {win} | {halted} |".format(
                name=instrument,
                ret=fmt_signed(value["return"]),
                dd=fmt(value["max_drawdown"]),
                trades=value["trades"],
                pf=fmt(value["profit_factor"]),
                win=fmt(value["win_rate"]),
                halted=value["halted_folds"],
            )
        )
    pooled = document["pooled"]
    lines += [
        "",
        f"- Average instrument return: {fmt_signed(pooled['average_instrument_return'])}",
        f"- Worst instrument drawdown: {fmt(pooled['worst_instrument_drawdown'])}",
        f"- Trades: {pooled['trades']}",
        f"- Pooled profit factor: {fmt(pooled['profit_factor'])}",
        f"- Positive instruments: {pooled['positive_instruments']}",
        "",
        "## Criteria",
        "",
        "| Criterion | Result |",
        "|---|---|",
    ]
    for name, value in document["criteria"].items():
        lines.append(f"| {name} | {'pass' if value else 'fail'} |")
    lines.append("")
    return "\n".join(lines)


def fmt(value: float | None) -> str:
    if value is None:
        return "-"
    if math.isinf(value):
        return "inf"
    return f"{value:.4f}"


def fmt_signed(value: float | None) -> str:
    if value is None:
        return "-"
    return f"{value:+.4f}"


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    matrix = read_json(Path(args.matrix).expanduser())
    if not isinstance(matrix, dict) or not isinstance(matrix.get("cells"), list):
        print("error: matrix must be an object with cells", file=sys.stderr)
        return 1
    document = evaluate(matrix["cells"], args)
    document["partition"] = matrix.get("partition")
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "strategy_v9.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    (out / "strategy_v9.md").write_text(markdown(document), encoding="utf-8")
    print(f"ok strategy evaluation -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
