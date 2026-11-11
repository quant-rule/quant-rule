#!/usr/bin/env python3
"""Check a simulation rule on the unmarked 2020-2022 window.

The check is a descriptive event study, not a backtest. It reads the M1.3
bundle, applies the run's event and filter at the event's available bar, enters
at the next bar's open, exits at the close after the declared horizon, and
subtracts the cost proxy. It compares the filtered events with the unfiltered
event control and records the half-year direction.

Usage:
    python3 tools/simulation/check.py \
      --check-config data/simulation/2026-11-10/SIM-02-level-rejection/check-config.json \
      --out data/simulation/2026-11-10/SIM-02-level-rejection
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from bundle import BundleError, load_bundles, load_catalog, parse_utc
from window_select import SelectError

DEFAULT_REGISTRY = Path("docs/registry/indicators.json")
CHECK_SCHEMA = "quantrule.simulation-check/v1"
MIN_EVENTS = 30
MIN_HALF_YEAR_EVENTS = 5
MIN_POSITIVE_HALF_YEARS = 3


class CheckError(Exception):
    """The check configuration or its inputs are invalid."""


def load_config(path: Path, repo_root: Path) -> dict[str, Any]:
    """Read and validate the check configuration."""
    config = json.loads(path.read_text(encoding="utf-8"))
    if config.get("schema") != CHECK_SCHEMA:
        raise CheckError(f"{path} has an unknown schema {config.get('schema')!r}")
    if not config.get("bundles"):
        raise CheckError("the check config must list bundles")
    base = path.parent
    bundles = []
    for item in config["bundles"]:
        bundle = Path(item)
        if not bundle.is_absolute():
            bundle = (base / bundle).resolve()
        bundles.append(bundle)
    config["_bundles"] = bundles
    registry = Path(config.get("registry", DEFAULT_REGISTRY))
    if not registry.is_absolute():
        registry = (repo_root / registry).resolve()
    config["_registry"] = registry
    return config


def resolve_event(name_or_code: Any, catalog: dict[int, str]) -> int:
    """Resolve a registry event name or code."""
    if isinstance(name_or_code, int):
        if name_or_code in catalog:
            return name_or_code
        raise CheckError(f"event code {name_or_code} is not in the registry")
    name = str(name_or_code)
    for code, label in catalog.items():
        if label == name:
            return code
    raise CheckError(f"unknown event {name!r}")


def event_rows(
    bundle,
    event_code: int,
    from_ts: int,
    to_ts: int,
    horizon_bars: int,
    confirmation_bars: int = 0,
) -> list[dict[str, Any]]:
    """Return one forward-path row per event in the check window.

    ``confirmation_bars`` delays the entry past the event so a filter may read a
    confirmation bar (for example, the bar that closes back inside a level).
    """
    rows = []
    for event in bundle.events:
        if event.code != event_code:
            continue
        if event.available_bar not in bundle.bar_index:
            continue
        index = bundle.index_of(event.available_bar)
        event_bar = bundle.bars[index]
        if not from_ts <= event_bar.timestamp <= to_ts:
            continue
        confirmation_index = index + 1
        entry_index = index + 1 + confirmation_bars
        exit_index = entry_index + horizon_bars
        if entry_index >= len(bundle.bars) or exit_index >= len(bundle.bars):
            continue
        entry = bundle.bars[entry_index]
        exit_bar = bundle.bars[exit_index]
        rows.append(
            {
                "event_bar": event_bar,
                "direction": event.direction,
                "reference": event.reference,
                "confirmation_bar": bundle.bars[confirmation_index] if confirmation_bars else None,
                "entry_index": entry_index,
                "entry_time": entry.timestamp,
                "exit_index": exit_index,
                "entry_open": entry.open,
                "exit_close": exit_bar.close,
            }
        )
    rows.sort(key=lambda row: (row["entry_time"], row["entry_index"]))
    return rows


def _return(row: dict[str, Any], side: str, cost_bps: float) -> float:
    entry = row["entry_open"]
    exit_price = row["exit_close"]
    gross = (exit_price - entry) / entry if side == "long" else (entry - exit_price) / entry
    return gross - cost_bps / 10_000.0


def _half_year(timestamp: int) -> str:
    moment = datetime.fromtimestamp(timestamp, tz=timezone.utc)
    return f"{moment.year}H{1 if moment.month <= 6 else 2}"


def _stats(rows: list[dict[str, Any]], side: str, cost_bps: float) -> dict[str, Any]:
    if not rows:
        return {"n": 0, "mean_net": None, "hit_rate": None, "half_years": []}
    returns = [_return(row, side, cost_bps) for row in rows]
    buckets: dict[str, list[float]] = {}
    for row, value in zip(rows, returns):
        buckets.setdefault(_half_year(row["entry_time"]), []).append(value)
    half_years = [
        {
            "bucket": bucket,
            "n": len(values),
            "mean_net": mean(values),
        }
        for bucket, values in sorted(buckets.items())
    ]
    positive = sum(
        1
        for item in half_years
        if item["n"] >= MIN_HALF_YEAR_EVENTS and item["mean_net"] > 0
    )
    return {
        "n": len(returns),
        "mean_net": mean(returns),
        "hit_rate": sum(1 for value in returns if value > 0) / len(returns),
        "half_years": half_years,
        "positive_half_years": positive,
    }


def _non_overlapping(rows: list[dict[str, Any]], horizon_bars: int) -> list[dict[str, Any]]:
    selected = []
    last_exit = -1
    for row in rows:
        if row["entry_index"] <= last_exit:
            continue
        selected.append(row)
        last_exit = row["exit_index"]
    return selected


def _passes_direction(row: dict[str, Any], side: str, side_config: dict[str, Any]) -> bool:
    """Apply the optional direction mapping for an event-driven side."""
    direction = row.get("direction")
    if direction is None:
        return True
    if side_config.get("direction_only"):
        if direction == 0:
            return False
        return (side == "long" and direction > 0) or (side == "short" and direction < 0)
    if side_config.get("opposite_direction"):
        if direction == 0:
            return False
        return (side == "long" and direction < 0) or (side == "short" and direction > 0)
    return True


def _passes_filter(row: dict[str, Any], side: str, side_config: dict[str, Any]) -> bool:
    """Apply the event filters after the direction mapping."""
    if not _passes_direction(row, side, side_config):
        return False
    ema_stack = side_config.get("ema_stack")
    if ema_stack is not None and row["event_bar"].ema_stack != int(ema_stack):
        return False
    bucket = side_config.get("atr_bucket")
    if bucket is not None:
        ratio = row["event_bar"].atr_ratio
        if ratio is None:
            return False
        if (bucket == "ge1") != (ratio >= 1.0):
            return False
    rvol_min = side_config.get("rvol_min")
    if rvol_min is not None:
        rvol = row["event_bar"].rvol
        if rvol is None or rvol < float(rvol_min):
            return False
    column = str(side_config.get("wick_column", ""))
    minimum = side_config.get("wick_min")
    if minimum is not None:
        value = row["event_bar"].value(column)
        if value is None or value < float(minimum):
            return False
    close_min = side_config.get("close_min")
    close_max = side_config.get("close_max")
    close_position = row["event_bar"].value("close_position")
    if close_min is not None and (close_position is None or close_position < float(close_min)):
        return False
    if close_max is not None and (close_position is None or close_position > float(close_max)):
        return False
    if side_config.get("close_back"):
        confirmation = row.get("confirmation_bar")
        reference = row.get("reference")
        direction = row.get("direction")
        if confirmation is None or reference is None or direction in (None, 0):
            return False
        if direction > 0 and not confirmation.close < reference:
            return False
        if direction < 0 and not confirmation.close > reference:
            return False
    return True


def check_side(
    rows: list[dict[str, Any]],
    side: str,
    side_config: dict[str, Any],
    horizon_bars: int,
    cost_bps: float,
) -> dict[str, Any]:
    """Check one side and compare the filtered set with the event control."""
    candidates = _non_overlapping(rows, horizon_bars)
    direction_rows = [row for row in candidates if _passes_direction(row, side, side_config)]
    filtered_rows = [row for row in direction_rows if _passes_filter(row, side, side_config)]
    control = _stats(direction_rows, side, cost_bps)
    filtered = _stats(filtered_rows, side, cost_bps)
    underpowered = filtered["n"] < MIN_EVENTS or control["n"] < MIN_EVENTS
    survives = (
        not underpowered
        and filtered["mean_net"] > 0
        and filtered["hit_rate"] > 0.5
        and filtered["mean_net"] > control["mean_net"]
        and filtered["hit_rate"] > control["hit_rate"]
        and filtered["positive_half_years"] >= MIN_POSITIVE_HALF_YEARS
    )
    verdict = "underpowered" if underpowered else ("survives" if survives else "no_pattern")
    return {
        "side": side,
        "filter": side_config,
        "filtered": filtered,
        "control": control,
        "underpowered": underpowered,
        "verdict": verdict,
    }


def build(config: dict[str, Any], out: Path, repo_root: Path) -> dict[str, Any]:
    """Run the check and write ``check.json`` and ``check.md``."""
    catalog = load_catalog(config["_registry"])
    event_code = resolve_event(config.get("event"), catalog)
    from_ts = parse_utc(str(config["from"]))
    to_ts = parse_utc(str(config["to"]))
    horizon_bars = int(config.get("horizon_bars", 20))
    cost_bps = float(config.get("cost_bps", 6.0))
    bundle = load_bundles(
        config["_bundles"],
        900,
        from_ts,
        to_ts,
        data_version=config.get("data_version"),
    )
    confirmation_bars = int(config.get("confirmation_bars", 0))
    rows = event_rows(
        bundle, event_code, from_ts, to_ts, horizon_bars, confirmation_bars
    )
    sides = config.get("sides", {})
    if not sides:
        raise CheckError("the check config must declare at least one side")
    results = [
        check_side(rows, side, side_config, horizon_bars, cost_bps)
        for side, side_config in sorted(sides.items())
    ]
    verdicts = {result["verdict"] for result in results}
    if "survives" in verdicts:
        overall = "survives"
    elif verdicts == {"underpowered"}:
        overall = "underpowered"
    else:
        overall = "no_pattern"
    record = {
        "schema": CHECK_SCHEMA,
        "run_id": config.get("run_id", "unknown"),
        "event": catalog.get(event_code, str(event_code)),
        "event_code": event_code,
        "horizon_bars": horizon_bars,
        "confirmation_bars": confirmation_bars,
        "cost_bps": cost_bps,
        "from": config["from"],
        "to": config["to"],
        "data_version": bundle.data_version,
        "model": "M1.3",
        "registry_version": "2026-11-10.2",
        "thresholds": {
            "min_events": MIN_EVENTS,
            "min_positive_half_years": MIN_POSITIVE_HALF_YEARS,
        },
        "candidate_count": len(rows),
        "results": results,
        "verdict": overall,
        "claim": "screen only; not an edge; no validation or holdout access",
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "check.json").write_text(
        json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    lines = [
        f"# Check — {record['run_id']}",
        "",
        f"- Event: `{record['event']}`",
        f"- Window: {record['from']} to {record['to']}",
        f"- Horizon: {horizon_bars} bars; cost proxy: {cost_bps} bps round trip",
        f"- Model: `M1.3`; registry `2026-11-10.2`",
        f"- Verdict: **{overall}**",
        "- Claim: screen only; not an edge; no validation or holdout access",
        "",
        "| Side | Filter | Filtered n | Mean net | Hit rate | Control n | Control mean | Control hit | Positive half-years | Verdict |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for result in results:
        filtered = result["filtered"]
        control = result["control"]
        lines.append(
            f"| {result['side']} | `{result['filter']}` | {filtered['n']} | "
            f"{_fmt(filtered['mean_net'])} | {_fmt_pct(filtered['hit_rate'])} | "
            f"{control['n']} | {_fmt(control['mean_net'])} | {_fmt_pct(control['hit_rate'])} | "
            f"{filtered.get('positive_half_years', 0)} | {result['verdict']} |"
        )
    lines += [
        "",
        "## Half-years (filtered)",
        "",
        "| Side | Half-year | n | Mean net |",
        "|---|---|---:|---:|",
    ]
    for result in results:
        for item in result["filtered"].get("half_years", []):
            lines.append(
                f"| {result['side']} | {item['bucket']} | {item['n']} | {_fmt(item['mean_net'])} |"
            )
    lines += [
        "",
        "The check is a screen, not a backtest. A survivor goes to Strategy",
        "Research with costs, a frozen config, and a pre-registration.",
        "",
    ]
    (out / "check.md").write_text("\n".join(lines), encoding="utf-8")
    return record


def _fmt(value: float | None) -> str:
    return "—" if value is None else f"{value:+.4%}"


def _fmt_pct(value: float | None) -> str:
    return "—" if value is None else f"{value:.1%}"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check-config", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args(argv[1:])
    repo_root = Path(__file__).resolve().parents[2]
    try:
        config = load_config(args.check_config.resolve(), repo_root)
        result = build(config, args.out.resolve(), repo_root)
    except (CheckError, BundleError, SelectError, KeyError, TypeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"check {result['run_id']}: {result['verdict']}")
    for item in result["results"]:
        print(
            f"  {item['side']}: filtered n={item['filtered']['n']} "
            f"mean={item['filtered']['mean_net']} control n={item['control']['n']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
