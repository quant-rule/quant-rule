#!/usr/bin/env python3
"""Compare five exit rules on the same entries (SIM-03).

The lab is descriptive. It takes a fixed entry set, simulates a fixed 2R
bracket, an EMA20 trail, a swing trail, a time stop, and a 1R partial, and
compares the captured part of the forward move. It never changes the entry and
never calls the result an edge.

Usage:
    python3 tools/simulation/exit_lab.py \
      --exit-config data/simulation/2026-11-10/SIM-03-exit-lab/exit-config.json \
      --out data/simulation/2026-11-10/SIM-03-exit-lab
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from statistics import mean
from typing import Any

from bundle import BundleError, load_bundles, parse_utc
from data_root import (
    DataRootError,
    current_worktree,
    guard_root,
    is_inside,
    resolve_root,
    substitute,
)
from marks import MarkError, read_marks

EXIT_SCHEMA = "quantrule.simulation-exit/v1"
RULES = ("fixed_2r", "ema20_trail", "swing_trail", "time", "partial_1r")
REQUIRED = {"schema", "run_id", "bundles", "from", "to", "check_from", "check_to"}


class ExitError(Exception):
    """The exit-lab configuration or its inputs is invalid."""


def load_config(path: Path, data_root: Path) -> dict[str, Any]:
    """Read and resolve the exit-lab configuration."""
    config = json.loads(path.read_text(encoding="utf-8"))
    missing = REQUIRED - set(config)
    if missing:
        raise ExitError(f"{path} is missing fields {sorted(missing)}")
    if config["schema"] != EXIT_SCHEMA:
        raise ExitError(f"{path} has an unknown schema {config['schema']!r}")
    base = path.parent
    bundles = []
    for item in config["bundles"]:
        bundle = Path(substitute(item, data_root))
        if not bundle.is_absolute():
            bundle = (base / bundle).resolve()
        bundles.append(bundle)
    config["_bundles"] = bundles
    config["_data_root"] = str(data_root)
    if "marks" in config:
        marks = Path(substitute(config["marks"], data_root))
        if not marks.is_absolute():
            marks = (base / marks).resolve()
        config["_marks"] = marks
    return config


def _entry_index(bundle, bar_id: int) -> int | None:
    try:
        return bundle.index_of(bar_id)
    except BundleError:
        return None


def entries_from_marks(bundle, marks: list[dict[str, str]], from_ts: int, to_ts: int) -> list[dict[str, Any]]:
    """Use the marked entries as the fixed entry set."""
    entries = []
    for mark in marks:
        if mark.get("action") != "entry":
            continue
        try:
            index = _entry_index(bundle, int(mark["bar"]))
        except (KeyError, ValueError):
            continue
        if index is None:
            continue
        bar = bundle.bars[index]
        if not from_ts <= bar.timestamp <= to_ts:
            continue
        entries.append({"index": index, "side": mark["side"], "source": "mark"})
    return entries


def entries_from_events(bundle, event_code: int, from_ts: int, to_ts: int) -> list[dict[str, Any]]:
    """Use every event bar as the same entry for both sides."""
    entries = []
    for event in bundle.events:
        if event.code != event_code or event.available_bar not in bundle.bar_index:
            continue
        index = bundle.index_of(event.available_bar)
        bar = bundle.bars[index]
        if not from_ts <= bar.timestamp <= to_ts:
            continue
        entries.append({"index": index, "side": "long", "source": "event"})
        entries.append({"index": index, "side": "short", "source": "event"})
    entries.sort(key=lambda row: (row["index"], row["side"]))
    return entries


def _non_overlapping(entries: list[dict[str, Any]], horizon: int) -> list[dict[str, Any]]:
    selected = []
    last_by_side: dict[str, int] = {}
    for entry in entries:
        side = entry["side"]
        last = last_by_side.get(side, -1)
        if entry["index"] <= last:
            continue
        selected.append(entry)
        last_by_side[side] = entry["index"] + horizon
    return selected


def _return(entry: float, exit_price: float, side: str, cost_bps: float) -> float:
    gross = (exit_price - entry) / entry if side == "long" else (entry - exit_price) / entry
    return gross - cost_bps / 10_000.0


def simulate(bundle, index: int, side: str, spec: dict[str, Any]) -> dict[str, Any] | None:
    """Simulate the five exits from one entry bar close."""
    bar = bundle.bars[index]
    if bar.atr is None or bar.atr <= 0:
        return None
    entry = bar.close
    atr = bar.atr
    horizon = int(spec.get("horizon_bars", 60))
    target = float(spec.get("target_atr", 2.0))
    stop = float(spec.get("stop_atr", 1.0))
    partial = float(spec.get("partial_atr", 1.0))
    cost = float(spec.get("cost_bps", 6.0))
    end = min(index + horizon, len(bundle.bars) - 1)
    if end <= index:
        return None
    sign = 1.0 if side == "long" else -1.0
    mfe = 0.0
    mfe_index = index
    for position in range(index + 1, end + 1):
        candidate = bundle.bars[position]
        if side == "long":
            favorable = candidate.high - entry
        else:
            favorable = entry - candidate.low
        if favorable > mfe:
            mfe = favorable
            mfe_index = position
    mfe_atr = mfe / atr
    mfe_return = mfe_atr * atr / entry
    results = {}

    # Fixed 2R bracket, stop-first on an equal-bar tie.
    exit_price = None
    exit_index = end
    for position in range(index + 1, end + 1):
        candidate = bundle.bars[position]
        if side == "long":
            if candidate.low <= entry - stop * atr:
                exit_price = entry - stop * atr
            elif candidate.high >= entry + target * atr:
                exit_price = entry + target * atr
        else:
            if candidate.high >= entry + stop * atr:
                exit_price = entry + stop * atr
            elif candidate.low <= entry - target * atr:
                exit_price = entry - target * atr
        if exit_price is not None:
            exit_index = position
            break
    if exit_price is None:
        exit_price = bundle.bars[end].close
    results["fixed_2r"] = _result(entry, exit_price, side, cost, mfe_return, exit_index, bundle)

    # EMA20 trail.
    exit_price = None
    exit_index = end
    for position in range(index + 1, end + 1):
        candidate = bundle.bars[position]
        if candidate.ema_fast is None:
            continue
        if (side == "long" and candidate.close < candidate.ema_fast) or (
            side == "short" and candidate.close > candidate.ema_fast
        ):
            exit_price = candidate.close
            exit_index = position
            break
    if exit_price is None:
        exit_price = bundle.bars[end].close
    results["ema20_trail"] = _result(entry, exit_price, side, cost, mfe_return, exit_index, bundle)

    # Swing trail at the last confirmed pivot.
    exit_price = None
    exit_index = end
    for position in range(index + 1, end + 1):
        candidate = bundle.bars[position]
        level = (
            candidate.value("last_pivot_low")
            if side == "long"
            else candidate.value("last_pivot_high")
        )
        if level is None:
            continue
        if (side == "long" and candidate.close < level) or (
            side == "short" and candidate.close > level
        ):
            exit_price = candidate.close
            exit_index = position
            break
    if exit_price is None:
        exit_price = bundle.bars[end].close
    results["swing_trail"] = _result(entry, exit_price, side, cost, mfe_return, exit_index, bundle)

    # Time stop.
    results["time"] = _result(
        entry, bundle.bars[end].close, side, cost, mfe_return, end, bundle
    )

    # 1R partial, remainder at 2R / stop / time.
    half_done = False
    realized = 0.0
    exit_index = end
    broke = False
    for position in range(index + 1, end + 1):
        candidate = bundle.bars[position]
        if side == "long":
            if candidate.low <= entry - stop * atr:
                stop_return = _return(entry, entry - stop * atr, side, 0.0)
                realized = (realized + 0.5 * stop_return) if half_done else stop_return
                exit_index = position
                broke = True
                break
            if not half_done and candidate.high >= entry + partial * atr:
                realized += 0.5 * _return(entry, entry + partial * atr, side, 0.0)
                half_done = True
            if candidate.high >= entry + target * atr:
                target_return = _return(entry, entry + target * atr, side, 0.0)
                realized = (realized + 0.5 * target_return) if half_done else target_return
                exit_index = position
                broke = True
                break
        else:
            if candidate.high >= entry + stop * atr:
                stop_return = _return(entry, entry + stop * atr, side, 0.0)
                realized = (realized + 0.5 * stop_return) if half_done else stop_return
                exit_index = position
                broke = True
                break
            if not half_done and candidate.low <= entry - partial * atr:
                realized += 0.5 * _return(entry, entry - partial * atr, side, 0.0)
                half_done = True
            if candidate.low <= entry - target * atr:
                target_return = _return(entry, entry - target * atr, side, 0.0)
                realized = (realized + 0.5 * target_return) if half_done else target_return
                exit_index = position
                broke = True
                break
    if not broke:
        if half_done:
            realized += 0.5 * _return(entry, bundle.bars[end].close, side, 0.0)
        else:
            realized = _return(entry, bundle.bars[end].close, side, 0.0)
    results["partial_1r"] = _result(
        entry,
        bundle.bars[exit_index].close,
        side,
        cost,
        mfe_return,
        exit_index,
        bundle,
        forced_return=realized - cost / 10_000.0,
    )
    return results


def _result(
    entry: float,
    exit_price: float,
    side: str,
    cost_bps: float,
    mfe_return: float,
    exit_index: int,
    bundle,
    forced_return: float | None = None,
) -> dict[str, Any]:
    value = forced_return if forced_return is not None else _return(entry, exit_price, side, cost_bps)
    captured = value / mfe_return if mfe_return > 0 else None
    return {
        "return_pct": value,
        "captured": captured,
        "exit_index": exit_index,
        "exit_bar": bundle.bars[exit_index].bar,
    }


def aggregate(rows: list[dict[str, Any]], label: str) -> list[dict[str, Any]]:
    """Aggregate per side and rule."""
    groups: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for row in rows:
        for rule, result in row["results"].items():
            groups.setdefault((row["side"], rule), []).append(result)
    table = []
    for (side, rule), results in sorted(groups.items()):
        returns = [result["return_pct"] for result in results]
        captured = [result["captured"] for result in results if result["captured"] is not None]
        table.append(
            {
                "set": label,
                "side": side,
                "rule": rule,
                "n": len(results),
                "mean_return": mean(returns),
                "hit_rate": sum(1 for value in returns if value > 0) / len(returns),
                "mean_captured": mean(captured) if captured else None,
            }
        )
    return table


def _fmt(value: float | None, percent: bool = False) -> str:
    if value is None:
        return "—"
    return f"{value:+.2%}" if percent else f"{value:.2f}"


def build(config: dict[str, Any], out: Path) -> dict[str, Any]:
    """Run the exit comparison and write the records."""
    label_from = parse_utc(str(config["from"]))
    label_to = parse_utc(str(config["to"]))
    check_from = parse_utc(str(config["check_from"]))
    check_to = parse_utc(str(config["check_to"]))
    horizon = int(config.get("horizon_bars", 60))
    bundle = load_bundles(
        config["_bundles"], 900, None, None, data_version=config.get("data_version")
    )
    if "marks" in config:
        marks = read_marks(config["_marks"])
        label_entries = entries_from_marks(bundle, marks, label_from, label_to)
    else:
        label_entries = entries_from_events(
            bundle, int(config.get("event_code", 149)), label_from, label_to
        )
    check_entries = entries_from_events(
        bundle, int(config.get("event_code", 149)), check_from, check_to
    )
    label_rows = []
    for entry in label_entries:
        results = simulate(bundle, entry["index"], entry["side"], config)
        if results:
            label_rows.append({"side": entry["side"], "results": results})
    check_rows = []
    for entry in _non_overlapping(check_entries, horizon):
        results = simulate(bundle, entry["index"], entry["side"], config)
        if results:
            check_rows.append({"side": entry["side"], "results": results})
    label_table = aggregate(label_rows, "label")
    check_table = aggregate(check_rows, "check")
    lines = [
        f"# {config['run_id']} — Exit lab",
        "",
        f"- Model: `M1.3`; registry `2026-11-10.2`",
        f"- Label entries: {len(label_rows)}; check entries: {len(check_rows)}",
        f"- Horizon: {horizon} bars; cost proxy: {config.get('cost_bps', 6)} bps",
        "- Claim: descriptive exit comparison; no edge claim",
        "",
        "## Label set",
        "",
        "| Side | Rule | n | Mean return | Hit rate | Mean captured |",
        "|---|---|---:|---:|---:|---:|",
    ]
    for row in label_table:
        lines.append(
            f"| {row['side']} | {row['rule']} | {row['n']} | {_fmt(row['mean_return'], True)} | "
            f"{_fmt(row['hit_rate'] * 100)}% | {_fmt(row['mean_captured'])} |"
        )
    lines += [
        "",
        "## Check set (2020–2022, non-overlapping)",
        "",
        "| Side | Rule | n | Mean return | Hit rate | Mean captured |",
        "|---|---|---:|---:|---:|---:|",
    ]
    for row in check_table:
        lines.append(
            f"| {row['side']} | {row['rule']} | {row['n']} | {_fmt(row['mean_return'], True)} | "
            f"{_fmt(row['hit_rate'] * 100)}% | {_fmt(row['mean_captured'])} |"
        )
    lines += [
        "",
        "## Reading",
        "",
        "- The comparison holds the entries fixed; it asks only which exit keeps more of the move.",
        "- A positive exit on a negative entry set is not an edge; it means the exit is less bad.",
        "- The check is descriptive and does not use validation or holdout data.",
        "",
    ]
    out.mkdir(parents=True, exist_ok=True)
    (out / "exit-lab.md").write_text("\n".join(lines), encoding="utf-8")
    record = {
        "schema": EXIT_SCHEMA,
        "run_id": config["run_id"],
        "model": "M1.3",
        "registry_version": "2026-11-10.2",
        "data_version": bundle.data_version,
        "data_root": config.get("_data_root"),
        "label_window": {"from": config["from"], "to": config["to"]},
        "check_window": {"from": config["check_from"], "to": config["check_to"]},
        "horizon_bars": horizon,
        "cost_bps": config.get("cost_bps", 6.0),
        "label_entries": len(label_rows),
        "check_entries": len(check_rows),
        "label_table": label_table,
        "check_table": check_table,
        "claim": "descriptive exit comparison; no edge claim",
    }
    (out / "exit-lab.json").write_text(
        json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out / "manifest.json").write_text(
        json.dumps(
            {
                "schema": "quantrule.simulation-run/v1",
                "run_id": config["run_id"],
                "kind": "exit-lab",
                "model": "M1.3",
                "registry_version": "2026-11-10.2",
                "data_version": bundle.data_version,
                "label_window": record["label_window"],
                "check_window": record["check_window"],
                "label_entries": len(label_rows),
                "check_entries": len(check_rows),
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
                "- Human acceptance: pending",
                "",
                "## Decision",
                "",
                "Record the exit comparison and decide whether the exit, rather than the entry, is the weak part.",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return record


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exit-config", required=True, type=Path)
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
            raise ExitError(
                f"the output {out} is inside the current worktree; "
                "write to the external data root or pass --allow-worktree-data"
            )
        config = load_config(args.exit_config.resolve(), data_root)
        result = build(config, out)
    except (ExitError, BundleError, MarkError, DataRootError, KeyError, TypeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"exit lab {result['run_id']}: {result['label_entries']} label, {result['check_entries']} check")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
