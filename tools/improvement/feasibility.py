#!/usr/bin/env python3
"""Feasibility preflight for a strategy hypothesis.

The preflight runs before the first backtest cell. It answers one question:
can the declared stop, target, timeframe, and round-trip cost produce a
positive expectancy without requiring a win rate above 50%?

It reads only the strategy config, the data config, and the raw candles. It
does not read a backtest result, a fill, or a P&L, so it cannot peek at the
outcome. The calculation is deterministic:

    cost_atr = cost_bps / median(ATR / close * 10_000)
    net_target_atr = target_atr - cost_atr
    net_stop_atr = stop_atr + cost_atr
    breakeven_win_rate = net_stop_atr / (net_stop_atr + net_target_atr)

A hypothesis passes only when the target is above the cost and the breakeven
win rate is at most 50%. A hypothesis that needs a win rate above 50% is
rejected before any backtest compute is spent.

The ATR is a local Wilder ATR(14) estimate from the raw candles, not a
reproduction of the Rust model. The screen is deliberately conservative and
uses the median ATR; the 25th percentile is reported as a warning for quiet
markets.

Usage:
    python3 tools/improvement/feasibility.py \
        --strategy config/strategy/sr1-fade-1m.json \
        --data config/sr1/runs/btc-2018/data.json \
        --cost-bps 6 \
        --out /tmp/feasibility
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from dataclasses import dataclass
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Iterable

SCHEMA = "quantrule.feasibility-report/v1"
ATR_PERIOD = 14
MAX_TIMEFRAME_SECS = 3_600
DEFAULT_MAX_BARS = 2_000_000
MIN_ATR_BARS = 30
BREAKEVEN_LIMIT = 0.50


class FeasibilityError(ValueError):
    """A configuration or input cannot be evaluated."""


@dataclass(frozen=True)
class Bar:
    timestamp: int
    open: float
    high: float
    low: float
    close: float
    volume: float


@dataclass(frozen=True)
class StrategyRisk:
    strategy_id: str
    engine: str
    execution_timeframe: str
    execution_secs: int
    context_timeframes: tuple[str, ...]
    context_secs: tuple[int, ...]
    stop_atr: float
    target_atr: float


@dataclass(frozen=True)
class DataSpec:
    path: Path
    instrument: str
    source_timeframe: str
    source_secs: int
    available_timeframes: tuple[str, ...]
    start: date | None
    end: date | None


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise FeasibilityError(f"cannot read {path}: {error}") from None
    except json.JSONDecodeError as error:
        raise FeasibilityError(f"{path}: invalid JSON: {error.msg}") from None


def parse_timeframe(code: Any, path: str) -> int:
    if not isinstance(code, str) or not code:
        raise FeasibilityError(f"{path} must be a non-empty timeframe code")
    units = {"s": 1, "m": 60, "h": 3_600}
    suffix = code[-1]
    if suffix not in units:
        raise FeasibilityError(
            f"{path} must end in s, m, or h (for example 1m, 15m, 1h)"
        )
    value = code[:-1]
    if not value.isdigit() or int(value) <= 0:
        raise FeasibilityError(f"{path} must be a positive integer plus a unit")
    return int(value) * units[suffix]


def parse_date(value: Any, path: str) -> date | None:
    if value is None:
        return None
    if not isinstance(value, str):
        raise FeasibilityError(f"{path} must be a date string or null")
    try:
        return date.fromisoformat(value)
    except ValueError as error:
        raise FeasibilityError(f"{path} is not a valid date: {value!r}") from error


def load_strategy(path: Path) -> StrategyRisk:
    document = read_json(path)
    if not isinstance(document, dict):
        raise FeasibilityError("strategy config must be a JSON object")
    if document.get("schema") != "quantrule.strategy/v1":
        raise FeasibilityError("strategy config schema must be quantrule.strategy/v1")
    engine = document.get("engine")
    if not isinstance(engine, str) or not engine:
        raise FeasibilityError("$.engine must be a non-empty engine name")
    execution = document.get("execution_timeframe")
    execution_secs = parse_timeframe(execution, "$.execution_timeframe")
    raw_context = document.get("context_timeframes", [])
    if not isinstance(raw_context, list):
        raise FeasibilityError("$.context_timeframes must be a list")
    context: list[str] = []
    context_secs: list[int] = []
    for index, value in enumerate(raw_context):
        seconds = parse_timeframe(value, f"$.context_timeframes[{index}]")
        context.append(value)
        context_secs.append(seconds)
    risk = document.get("risk")
    if not isinstance(risk, dict):
        raise FeasibilityError("$.risk must be an object")
    stop_atr = risk.get("stop_atr_multiple")
    reward_to_risk = risk.get("reward_to_risk")
    for name, value in (("stop_atr_multiple", stop_atr), ("reward_to_risk", reward_to_risk)):
        if not isinstance(value, (int, float)) or isinstance(value, bool):
            raise FeasibilityError(f"$.risk.{name} must be a number")
        if not math.isfinite(value) or value <= 0.0:
            raise FeasibilityError(f"$.risk.{name} must be finite and positive")
    target_atr = float(stop_atr) * float(reward_to_risk)
    strategy_id = document.get("id")
    if not isinstance(strategy_id, str) or not strategy_id:
        strategy_id = path.stem
    return StrategyRisk(
        strategy_id=strategy_id,
        engine=engine,
        execution_timeframe=execution,
        execution_secs=execution_secs,
        context_timeframes=tuple(context),
        context_secs=tuple(context_secs),
        stop_atr=float(stop_atr),
        target_atr=target_atr,
    )


def load_data(path: Path) -> DataSpec:
    document = read_json(path)
    if not isinstance(document, dict):
        raise FeasibilityError("data config must be a JSON object")
    if document.get("schema") != "quantrule.data/v3":
        raise FeasibilityError("data config schema must be quantrule.data/v3")
    source = document.get("source")
    if not isinstance(source, dict) or source.get("kind") != "csv":
        raise FeasibilityError("$.source must be a csv source")
    source_path = source.get("path")
    if not isinstance(source_path, str) or not source_path:
        raise FeasibilityError("$.source.path must be a non-empty path")
    resolved = Path(source_path).expanduser()
    if not resolved.is_absolute():
        resolved = (path.parent / resolved).resolve()
    source_timeframe = document.get("source_timeframe")
    source_secs = parse_timeframe(source_timeframe, "$.source_timeframe")
    raw_available = document.get("available_timeframes")
    if not isinstance(raw_available, list) or not raw_available:
        raise FeasibilityError("$.available_timeframes must be a non-empty list")
    available = tuple(str(item) for item in raw_available)
    for index, value in enumerate(available):
        parse_timeframe(value, f"$.available_timeframes[{index}]")
    instrument = document.get("instrument")
    if not isinstance(instrument, str) or not instrument:
        raise FeasibilityError("$.instrument must be a non-empty string")
    start = parse_date(document.get("from"), "$.from")
    end = parse_date(document.get("to"), "$.to")
    return DataSpec(
        path=resolved,
        instrument=instrument,
        source_timeframe=source_timeframe,
        source_secs=source_secs,
        available_timeframes=available,
        start=start,
        end=end,
    )


def date_bounds(spec: DataSpec) -> tuple[int, int]:
    start = (
        datetime.combine(spec.start, datetime.min.time(), tzinfo=timezone.utc)
        if spec.start is not None
        else datetime.min.replace(tzinfo=timezone.utc)
    )
    end_date = spec.end + timedelta(days=1) if spec.end is not None else None
    end = (
        datetime.combine(end_date, datetime.min.time(), tzinfo=timezone.utc)
        if end_date is not None
        else datetime.max.replace(tzinfo=timezone.utc)
    )
    return int(start.timestamp()), int(end.timestamp())


def parse_timestamp(text: str) -> int:
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError as error:
        raise FeasibilityError(f"invalid candle timestamp {text!r}") from error
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return int(parsed.astimezone(timezone.utc).timestamp())


def read_bars(spec: DataSpec, target_secs: int, max_bars: int) -> tuple[list[Bar], bool]:
    if target_secs < spec.source_secs:
        raise FeasibilityError(
            "the execution timeframe is shorter than the source timeframe"
        )
    start, end = date_bounds(spec)
    bars: list[Bar] = []
    current: Bar | None = None
    current_bucket: int | None = None
    truncated = False
    with spec.path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                timestamp = parse_timestamp(row["Date"])
                open_price = float(row["Open"])
                high = float(row["High"])
                low = float(row["Low"])
                close = float(row["Close"])
                volume = float(row["Volume"]) if row.get("Volume") else 0.0
            except (KeyError, TypeError, ValueError):
                continue
            if timestamp < start or timestamp >= end:
                continue
            if (
                not all(
                    math.isfinite(value)
                    for value in (open_price, high, low, close, volume)
                )
                or close <= 0.0
                or high < low
            ):
                continue
            bucket = timestamp // target_secs * target_secs
            if current is None:
                current = Bar(bucket, open_price, high, low, close, volume)
                current_bucket = bucket
            elif bucket == current_bucket:
                current = Bar(
                    current.timestamp,
                    current.open,
                    max(current.high, high),
                    min(current.low, low),
                    close,
                    current.volume + volume,
                )
            else:
                bars.append(current)
                if len(bars) >= max_bars:
                    truncated = True
                    break
                current = Bar(bucket, open_price, high, low, close, volume)
                current_bucket = bucket
    if current is not None and len(bars) < max_bars:
        bars.append(current)
    bars.sort(key=lambda bar: bar.timestamp)
    return bars, truncated


def wilder_atr(bars: list[Bar], period: int = ATR_PERIOD) -> list[float]:
    if len(bars) < period + 1:
        return []
    true_ranges: list[float] = []
    previous_close = bars[0].close
    for bar in bars:
        high_low = bar.high - bar.low
        high_close = abs(bar.high - previous_close)
        low_close = abs(bar.low - previous_close)
        true_ranges.append(max(high_low, high_close, low_close))
        previous_close = bar.close
    atr = sum(true_ranges[1 : period + 1]) / period
    values = [atr]
    for true_range in true_ranges[period + 1 :]:
        atr = (atr * (period - 1) + true_range) / period
        values.append(atr)
    return values


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        raise FeasibilityError("cannot take a percentile of an empty list")
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = fraction * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def evaluate(
    strategy: StrategyRisk,
    data: DataSpec,
    cost_bps: float,
    max_bars: int = DEFAULT_MAX_BARS,
) -> dict[str, Any]:
    if not math.isfinite(cost_bps) or cost_bps < 0.0:
        raise FeasibilityError("--cost-bps must be finite and non-negative")
    reasons: list[str] = []
    if strategy.execution_secs > MAX_TIMEFRAME_SECS or any(
        seconds > MAX_TIMEFRAME_SECS for seconds in strategy.context_secs
    ):
        reasons.append("timeframe_above_1h")
    if strategy.execution_timeframe not in data.available_timeframes:
        reasons.append("timeframe_not_in_data")
    if data.source_secs > strategy.execution_secs:
        reasons.append("source_above_execution_timeframe")

    bars, truncated = read_bars(data, strategy.execution_secs, max_bars)
    if len(bars) < MIN_ATR_BARS + ATR_PERIOD:
        raise FeasibilityError(
            f"not enough bars for ATR({ATR_PERIOD}): {len(bars)}"
        )
    atr_values = wilder_atr(bars)
    atr_bps_values = [
        atr / bar.close * 10_000.0
        for atr, bar in zip(atr_values, bars[ATR_PERIOD + 1 :])
        if atr > 0.0 and bar.close > 0.0
    ]
    if len(atr_bps_values) < MIN_ATR_BARS:
        raise FeasibilityError(
            f"not enough ATR values: {len(atr_bps_values)}"
        )
    median_atr_bps = percentile(atr_bps_values, 0.50)
    p25_atr_bps = percentile(atr_bps_values, 0.25)
    cost_atr = cost_bps / median_atr_bps if median_atr_bps > 0.0 else math.inf
    net_target_atr = strategy.target_atr - cost_atr
    net_stop_atr = strategy.stop_atr + cost_atr
    if net_target_atr <= 0.0:
        reasons.append("target_below_cost")
        breakeven = None
    else:
        breakeven = net_stop_atr / (net_stop_atr + net_target_atr)
        if breakeven > BREAKEVEN_LIMIT:
            reasons.append("breakeven_above_50pct")
    p25_cost_atr = cost_bps / p25_atr_bps if p25_atr_bps > 0.0 else math.inf
    p25_breakeven = None
    if strategy.target_atr > p25_cost_atr:
        p25_net_target = strategy.target_atr - p25_cost_atr
        p25_net_stop = strategy.stop_atr + p25_cost_atr
        p25_breakeven = p25_net_stop / (p25_net_stop + p25_net_target)

    return {
        "schema": SCHEMA,
        "strategy_id": strategy.strategy_id,
        "engine": strategy.engine,
        "instrument": data.instrument,
        "execution_timeframe": strategy.execution_timeframe,
        "execution_timeframe_secs": strategy.execution_secs,
        "context_timeframes": list(strategy.context_timeframes),
        "context_timeframe_secs": list(strategy.context_secs),
        "max_timeframe_secs": MAX_TIMEFRAME_SECS,
        "cost_bps_round_trip": cost_bps,
        "stop_atr": strategy.stop_atr,
        "target_atr": strategy.target_atr,
        "bars": len(bars),
        "atr_values": len(atr_bps_values),
        "atr_bps_median": median_atr_bps,
        "atr_bps_p25": p25_atr_bps,
        "cost_atr": cost_atr,
        "net_target_atr": net_target_atr,
        "net_stop_atr": net_stop_atr,
        "breakeven_win_rate": breakeven,
        "p25_cost_atr": p25_cost_atr,
        "p25_breakeven_win_rate": p25_breakeven,
        "truncated": truncated,
        "verdict": "pass" if not reasons else "fail",
        "reasons": reasons,
    }


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strategy", required=True, help="strategy config JSON")
    parser.add_argument("--data", required=True, help="data config JSON")
    parser.add_argument(
        "--cost-bps", type=float, required=True, help="round-trip cost in bps"
    )
    parser.add_argument("--out", help="output directory or JSON file")
    parser.add_argument(
        "--max-bars",
        type=int,
        default=DEFAULT_MAX_BARS,
        help=f"maximum execution bars to read (default {DEFAULT_MAX_BARS})",
    )
    return parser.parse_args(argv)


def write_report(report: dict[str, Any], destination: Path) -> Path:
    path = destination / "feasibility.json" if destination.suffix != ".json" else destination
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def format_report(report: dict[str, Any]) -> str:
    breakeven = report["breakeven_win_rate"]
    breakeven_text = "n/a" if breakeven is None else f"{breakeven:.4f}"
    p25 = report["p25_breakeven_win_rate"]
    p25_text = "n/a" if p25 is None else f"{p25:.4f}"
    return "\n".join(
        (
            f"strategy: {report['strategy_id']} ({report['engine']})",
            f"instrument: {report['instrument']}",
            f"timeframe: {report['execution_timeframe']} (max 1h)",
            f"bars: {report['bars']} (truncated: {str(report['truncated']).lower()})",
            f"ATR({ATR_PERIOD}) median: {report['atr_bps_median']:.4f} bps",
            f"ATR({ATR_PERIOD}) p25: {report['atr_bps_p25']:.4f} bps",
            f"round-trip cost: {report['cost_bps_round_trip']:.2f} bps",
            f"cost in ATR: {report['cost_atr']:.4f}",
            f"stop / target: {report['stop_atr']:.4f} / {report['target_atr']:.4f} ATR",
            f"net target / stop: {report['net_target_atr']:.4f} / {report['net_stop_atr']:.4f} ATR",
            f"breakeven win rate: {breakeven_text} (limit {BREAKEVEN_LIMIT:.2f})",
            f"p25 breakeven: {p25_text}",
            f"verdict: {report['verdict']}",
            f"reasons: {', '.join(report['reasons']) if report['reasons'] else 'none'}",
        )
    )


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    try:
        strategy = load_strategy(Path(args.strategy).expanduser())
        data = load_data(Path(args.data).expanduser())
        report = evaluate(strategy, data, args.cost_bps, args.max_bars)
    except FeasibilityError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    if args.out:
        path = write_report(report, Path(args.out).expanduser())
        print(f"report: {path}")
    print(format_report(report))
    return 0 if report["verdict"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
