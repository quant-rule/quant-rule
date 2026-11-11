#!/usr/bin/env python3
"""Meta-label the human marks and check the context split.

SIM-01 takes the existing 2018-2019 human entries, labels each one good or bad
by its 50-bar forward return, and compares one context state (EMA stack) with
one volatility state (ATR ratio). The best cell with at least 30 marks becomes
the candidate split; the 2020-2022 event study checks whether that context
holds on bars that were not marked.

Usage:
    python3 tools/simulation/meta.py \
      --meta-config data/simulation/2026-11-10/SIM-01-meta-label/meta-config.json \
      --out data/simulation/2026-11-10/SIM-01-meta-label
"""

from __future__ import annotations

import argparse
import bisect
import csv
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from bundle import BundleError, load_bundles, load_catalog, parse_utc
from check import _non_overlapping, _stats
from data_root import (
    DataRootError,
    current_worktree,
    guard_root,
    is_inside,
    resolve_root,
    substitute,
)
from marks import MarkError, read_marks
from review import _context

META_SCHEMA = "quantrule.simulation-meta/v1"
MIN_CELL = 30
CHECK_HORIZON = 20
COST_BPS = 6.0
REQUIRED = {
    "schema",
    "run_id",
    "marks",
    "bundles",
    "from",
    "to",
    "check_from",
    "check_to",
}


class MetaError(Exception):
    """The meta-label configuration or its inputs are invalid."""


def load_config(path: Path, data_root: Path) -> dict[str, Any]:
    """Read and resolve the meta-label configuration."""
    config = json.loads(path.read_text(encoding="utf-8"))
    missing = REQUIRED - set(config)
    if missing:
        raise MetaError(f"{path} is missing fields {sorted(missing)}")
    if config["schema"] != META_SCHEMA:
        raise MetaError(f"{path} has an unknown schema {config['schema']!r}")
    base = path.parent
    bundles = []
    for item in config["bundles"]:
        bundle = Path(substitute(item, data_root))
        if not bundle.is_absolute():
            bundle = (base / bundle).resolve()
        bundles.append(bundle)
    marks = Path(substitute(config["marks"], data_root))
    if not marks.is_absolute():
        marks = (base / marks).resolve()
    config["_bundles"] = bundles
    config["_marks"] = marks
    config["_data_root"] = str(data_root)
    return config


def nearest_bar(bundle, timestamps: list[int], moment: int, max_distance: int = 450):
    """Return the bar index nearest to an ISO timestamp, or None."""
    position = bisect.bisect_left(timestamps, moment)
    candidates = [index for index in (position - 1, position) if 0 <= index < len(timestamps)]
    if not candidates:
        return None
    best = min(candidates, key=lambda index: abs(timestamps[index] - moment))
    if abs(timestamps[best] - moment) > max_distance:
        return None
    return best


def forward_50(bundle, index: int, side: str, horizon: int) -> dict[str, Any] | None:
    """Return the 50-bar path from the entry close."""
    entry = bundle.bars[index]
    end = min(index + horizon, len(bundle.bars) - 1)
    if end <= index or entry.atr is None or entry.atr <= 0:
        return None
    sign = 1.0 if side == "long" else -1.0
    mfe = mae = 0.0
    for position in range(index + 1, end + 1):
        bar = bundle.bars[position]
        if side == "long":
            favorable = bar.high - entry.close
            adverse = entry.close - bar.low
        else:
            favorable = entry.close - bar.low
            adverse = bar.high - entry.close
        mfe = max(mfe, favorable)
        mae = max(mae, adverse)
    return {
        "return_50": (bundle.bars[end].close - entry.close) / entry.close * sign,
        "mfe_50": mfe / entry.atr,
        "mae_50": mae / entry.atr,
    }


def atr_bucket(value: float | None) -> str:
    """Return the two-level volatility state used by SIM-01."""
    if value is None:
        return "unknown"
    return "ge1" if value >= 1.0 else "lt1"


def meta_rows(bundle, marks: list[dict[str, str]], catalog: dict[int, str], horizon: int) -> tuple[list[dict[str, Any]], int]:
    """Label every human entry by its forward path and record its context."""
    timestamps = [bar.timestamp for bar in bundle.bars]
    rows = []
    unmatched = 0
    for mark in marks:
        if mark.get("action") != "entry":
            continue
        try:
            moment = parse_utc(mark["marked_time_utc"])
        except (KeyError, ValueError):
            unmatched += 1
            continue
        index = nearest_bar(bundle, timestamps, moment)
        if index is None:
            unmatched += 1
            continue
        path = forward_50(bundle, index, mark["side"], horizon)
        if path is None:
            unmatched += 1
            continue
        bar = bundle.bars[index]
        context = _context(bundle, bar.bar, catalog)
        quality = "good" if path["return_50"] > 0 else "bad" if path["return_50"] < 0 else "neutral"
        rows.append(
            {
                "chart": mark.get("chart", ""),
                "marked_time_utc": mark.get("marked_time_utc", ""),
                "bar": bar.bar,
                "side": mark["side"],
                "quality": quality,
                **path,
                **context,
                "atr_bucket": atr_bucket(bar.atr_ratio),
            }
        )
    return rows, unmatched


def cell_table(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Group the marks by side, EMA stack, and ATR bucket."""
    groups: dict[tuple[str, str, str], list[dict[str, Any]]] = {}
    for row in rows:
        key = (row["side"], row["ema_stack"], row["atr_bucket"])
        groups.setdefault(key, []).append(row)
    table = []
    for (side, stack, bucket), group in sorted(groups.items()):
        returns = [row["return_50"] for row in group]
        table.append(
            {
                "side": side,
                "ema_stack": stack,
                "atr_bucket": bucket,
                "n": len(group),
                "mean_return": mean(returns),
                "hit_rate": sum(1 for value in returns if value > 0) / len(returns),
                "mean_mfe": mean(row["mfe_50"] for row in group),
                "mean_mae": mean(row["mae_50"] for row in group),
            }
        )
    return table


def best_cell(table: list[dict[str, Any]], minimum: int) -> dict[str, Any] | None:
    """Pick the highest-mean cell with enough marks."""
    eligible = [row for row in table if row["n"] >= minimum]
    if not eligible:
        return None
    return sorted(eligible, key=lambda row: (-row["mean_return"], -row["n"]))[0]


def check_bars(bundle, side: str, stack: str, bucket: str, from_ts: int, to_ts: int, horizon: int) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Build the filtered and control event rows for the check window."""
    filtered = []
    control = []
    for index, bar in enumerate(bundle.bars):
        if not from_ts <= bar.timestamp <= to_ts:
            continue
        entry_index = index + 1
        exit_index = entry_index + horizon
        if entry_index >= len(bundle.bars) or exit_index >= len(bundle.bars):
            continue
        row = {
            "event_bar": bar,
            "entry_index": entry_index,
            "exit_index": exit_index,
            "entry_time": bundle.bars[entry_index].timestamp,
            "entry_open": bundle.bars[entry_index].open,
            "exit_close": bundle.bars[exit_index].close,
        }
        control.append(row)
        if bar.ema_stack is not None and bar.atr_ratio is not None:
            if _stack_label(bar.ema_stack) == stack and atr_bucket(bar.atr_ratio) == bucket:
                filtered.append(row)
    return filtered, control


def _stack_label(code: int) -> str:
    return {-1: "bear", 0: "mixed", 1: "bull"}.get(code, "—")


def _check(
    bundle,
    cell: dict[str, Any],
    from_ts: int,
    to_ts: int,
    horizon: int,
) -> dict[str, Any]:
    filtered_rows, control_rows = check_bars(
        bundle,
        cell["side"],
        cell["ema_stack"],
        cell["atr_bucket"],
        from_ts,
        to_ts,
        horizon,
    )
    filtered = _stats(_non_overlapping(filtered_rows, horizon), cell["side"], COST_BPS)
    control = _stats(_non_overlapping(control_rows, horizon), cell["side"], COST_BPS)
    underpowered = filtered["n"] < MIN_CELL or control["n"] < MIN_CELL
    survives = (
        not underpowered
        and filtered["mean_net"] > 0
        and filtered["hit_rate"] > 0.5
        and filtered["mean_net"] > control["mean_net"]
        and filtered["hit_rate"] > control["hit_rate"]
        and filtered["positive_half_years"] >= 3
    )
    verdict = "underpowered" if underpowered else ("survives" if survives else "no_pattern")
    return {
        "cell": cell,
        "filtered": filtered,
        "control": control,
        "underpowered": underpowered,
        "verdict": verdict,
    }


def write_meta_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    """Write the per-mark meta-label table."""
    fields = [
        "chart",
        "marked_time_utc",
        "bar",
        "side",
        "quality",
        "return_50",
        "mfe_50",
        "mae_50",
        "ema_stack",
        "rsi_zone",
        "rsi",
        "atr_ratio",
        "atr_bucket",
        "adx",
        "close_position",
        "upper_wick",
        "lower_wick",
        "body_ratio",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row.get(field) for field in fields})


def _fmt(value: float | None, percent: bool = False) -> str:
    if value is None:
        return "—"
    return f"{value:+.2%}" if percent else f"{value:.2f}"


def build(config: dict[str, Any], out: Path) -> dict[str, Any]:
    """Run SIM-01 and write the meta and check records."""
    catalog = load_catalog(
        Path(__file__).resolve().parents[2] / "docs" / "registry" / "indicators.json"
    )
    label_from = parse_utc(str(config["from"]))
    label_to = parse_utc(str(config["to"]))
    check_from = parse_utc(str(config["check_from"]))
    check_to = parse_utc(str(config["check_to"]))
    horizon = int(config.get("horizon_bars", 50))
    bundle = load_bundles(
        config["_bundles"],
        900,
        None,
        None,
        data_version=config.get("data_version"),
    )
    marks = read_marks(config["_marks"])
    rows, unmatched = meta_rows(bundle, marks, catalog, horizon)
    rows = [row for row in rows if label_from <= parse_utc(row["marked_time_utc"]) <= label_to]
    table = cell_table(rows)
    cell = best_cell(table, int(config.get("min_cell", MIN_CELL)))
    check = None
    if cell is not None:
        check = _check(bundle, cell, check_from, check_to, CHECK_HORIZON)
    lines = [
        f"# {config['run_id']} — Meta-label the human marks",
        "",
        "- Model: `M1.3`; registry `2026-11-10.2`",
        f"- Human entries mapped: {len(rows)}; unmatched: {unmatched}",
        f"- Label horizon: {horizon} bars; check: {CHECK_HORIZON} bars at 6 bps",
        "- Claim: descriptive meta-label; no edge claim",
        "",
        "## Label distribution",
        "",
        "| Side | Quality | n | Mean 50-bar return | Mean MFE | Mean MAE |",
        "|---|---|---:|---:|---:|---:|",
    ]
    groups: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for row in rows:
        groups.setdefault((row["side"], row["quality"]), []).append(row)
    for (side, quality), group in sorted(groups.items()):
        lines.append(
            f"| {side} | {quality} | {len(group)} | {_fmt(mean(row['return_50'] for row in group), True)} | "
            f"{_fmt(mean(row['mfe_50'] for row in group))} | {_fmt(mean(row['mae_50'] for row in group))} |"
        )
    lines += [
        "",
        "## Context cells",
        "",
        "| Side | EMA stack | ATR bucket | n | Mean return | Hit rate | Mean MFE | Mean MAE |",
        "|---|---|---|---:|---:|---:|---:|---:|",
    ]
    for cell_row in table:
        lines.append(
            f"| {cell_row['side']} | {cell_row['ema_stack']} | {cell_row['atr_bucket']} | {cell_row['n']} | "
            f"{_fmt(cell_row['mean_return'], True)} | {_fmt(cell_row['hit_rate'] * 100)}% | "
            f"{_fmt(cell_row['mean_mfe'])} | {_fmt(cell_row['mean_mae'])} |"
        )
    lines += [
        "",
        "## Candidate split",
        "",
        f"- Best cell with at least {config.get('min_cell', MIN_CELL)} marks: `{cell}`"
        if cell
        else "- No cell reached the minimum sample.",
        "",
        "## Check on 2020–2022",
        "",
    ]
    if check:
        filtered = check["filtered"]
        control = check["control"]
        lines += [
            f"- Verdict: **{check['verdict']}**",
            f"- Filtered: n={filtered['n']}, mean net {_fmt(filtered['mean_net'], True)}, hit {_fmt(filtered['hit_rate'] * 100)}%, positive half-years {filtered.get('positive_half_years', 0)}",
            f"- Control: n={control['n']}, mean net {_fmt(control['mean_net'], True)}, hit {_fmt(control['hit_rate'] * 100)}%",
            "",
            "| Half-year | n | Mean net |",
            "|---|---:|---:|",
        ]
        for item in filtered.get("half_years", []):
            lines.append(f"| {item['bucket']} | {item['n']} | {_fmt(item['mean_net'], True)} |")
    else:
        lines.append("- Not run; no cell reached the minimum sample.")
    lines += [
        "",
        "## Limits",
        "",
        "- The human marks are all `good`; the meta-label is the forward outcome, not the human label.",
        "- The check is a context screen, not a backtest; it does not use the human entry timing.",
        "- No validation or holdout data was read.",
        "",
    ]
    out.mkdir(parents=True, exist_ok=True)
    (out / "meta.md").write_text("\n".join(lines), encoding="utf-8")
    write_meta_csv(out / "meta-marks.csv", rows)
    record = {
        "schema": META_SCHEMA,
        "run_id": config["run_id"],
        "model": "M1.3",
        "registry_version": "2026-11-10.2",
        "data_version": bundle.data_version,
        "data_root": config.get("_data_root"),
        "label_window": {"from": config["from"], "to": config["to"]},
        "check_window": {"from": config["check_from"], "to": config["check_to"]},
        "horizon_bars": horizon,
        "check_horizon_bars": CHECK_HORIZON,
        "cost_bps": COST_BPS,
        "human_entries": len(rows),
        "unmatched": unmatched,
        "cells": table,
        "candidate": cell,
        "check": check,
        "claim": "descriptive meta-label; no edge claim",
    }
    (out / "meta-check.json").write_text(
        json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out / "manifest.json").write_text(
        json.dumps(
            {
                "schema": "quantrule.simulation-run/v1",
                "run_id": config["run_id"],
                "kind": "meta-label",
                "model": "M1.3",
                "registry_version": "2026-11-10.2",
                "data_version": bundle.data_version,
                "label_window": record["label_window"],
                "check_window": record["check_window"],
                "human_entries": len(rows),
                "candidate": cell,
                "check_status": check["verdict"] if check else "not_run",
                "visual_review": "not_applicable",
                "holdout_access": "none",
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    (out / "decision.md").write_text(
        "\n".join(
            [
                f"# Decision — {config['run_id']}",
                "",
                "- Status: pending",
                f"- Check verdict: {check['verdict'] if check else 'not_run'}",
                "- Human acceptance: pending",
                "",
                "## Decision",
                "",
                "Continue / refine / stop after the human review.",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return record


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--meta-config", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--data-root", type=Path, help="persistent local data root")
    parser.add_argument(
        "--allow-worktree-data",
        action="store_true",
        help="allow data inside the current worktree (disposable runs only)",
    )
    args = parser.parse_args(argv[1:])
    try:
        data_root = resolve_root(args.data_root)
        guard_root(data_root, args.allow_worktree_data)
        out = args.out.resolve()
        if is_inside(out, current_worktree()) and not args.allow_worktree_data:
            raise MetaError(
                f"the output {out} is inside the current worktree; "
                "write to the external data root or pass --allow-worktree-data"
            )
        config = load_config(args.meta_config.resolve(), data_root)
        result = build(config, out)
    except (MetaError, BundleError, MarkError, DataRootError, KeyError, TypeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"meta {result['run_id']}: {result['human_entries']} entries")
    print(f"candidate: {result['candidate']}")
    if result["check"]:
        print(f"check: {result['check']['verdict']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
