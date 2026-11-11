#!/usr/bin/env python3
"""Measure the raw directional edge of a fresh-onset compression breakout.

The rule is fixed by the SR13 protocol:

- the event is `signal.compression_range_onset` on the 5m ladder;
- the onset publishes a range high and low at its bar;
- the first **strict** close outside either boundary within `window` bars is
  the breakout; the next onset supersedes an unfinished one;
- entry is the next bar open after the breakout close;
- forward returns are measured at fixed horizons with no stop, target, halt, or
  position state.

The tool is a diagnostic. It reads a completed model bundle, writes JSON and
Markdown, and never writes a model or strategy artifact. It is deterministic:
inputs are sorted, sums use `math.fsum`, and the random-direction control uses
an explicit seed.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import random
import statistics
from dataclasses import dataclass
from pathlib import Path

TIMEFRAME_SECS = 300
ONSET_TYPE = 3
ONSET_SUBTYPE = 9
ONSET_SOURCE = 5
COILING_CONTEXT_CODE = 2
DEFAULT_WINDOW = 3
DEFAULT_HORIZONS = (1, 3, 6, 12)
DEFAULT_PRIMARY = 6
DEFAULT_COST_BPS = 6.0
DEFAULT_STRESS_BPS = 9.0
DEFAULT_SEED = 0

# The protocol fixes a 20-bar cluster gap. A non-overlapping sample keeps one
# breakout per cluster so the forward windows do not overlap.
CLUSTER_GAP_BARS = 20
RANDOM_DRAWS = 200
Z95 = 1.96


@dataclass(frozen=True)
class Bar:
    """One 5m model bar with the close-beyond-range inputs."""

    index: int
    open: float
    high: float
    low: float
    close: float
    context_code: int | None


@dataclass(frozen=True)
class Onset:
    """One compression onset and its published range."""

    bar: int
    high: float
    low: float


@dataclass(frozen=True)
class Bundle:
    """One model bundle's 5m bars and onset events."""

    label: str
    bars: tuple[Bar, ...]
    onsets: tuple[Onset, ...]


@dataclass(frozen=True)
class Signal:
    """One reconstructed breakout and its forward returns in bps."""

    bundle: str
    onset_bar: int
    break_bar: int
    direction: int
    entry: float
    returns: dict[int, float]


def _float(value: str) -> float | None:
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        return None
    if not math.isfinite(parsed):
        return None
    return parsed


def _int(value: str) -> int | None:
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def parse_readings(path: Path) -> tuple[Bar, ...]:
    """Reads the 5m rows of a model bundle's readings CSV."""
    bars: list[Bar] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if _int(row.get("timeframe_secs", "")) != TIMEFRAME_SECS:
                continue
            index = _int(row.get("bar", ""))
            open_price = _float(row.get("open", ""))
            high = _float(row.get("high", ""))
            low = _float(row.get("low", ""))
            close = _float(row.get("close", ""))
            if index is None or None in (open_price, high, low, close):
                continue
            bars.append(
                Bar(
                    index=index,
                    open=open_price,
                    high=high,
                    low=low,
                    close=close,
                    context_code=_int(row.get("context_code", "")),
                )
            )
    bars.sort(key=lambda bar: bar.index)
    return tuple(bars)


def parse_onsets(path: Path) -> tuple[Onset, ...]:
    """Reads the compression-range onset events from a model bundle."""
    by_bar: dict[int, Onset] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if _int(row.get("timeframe_secs", "")) != TIMEFRAME_SECS:
                continue
            if _int(row.get("event_type_code", "")) != ONSET_TYPE:
                continue
            if _int(row.get("event_subtype_code", "")) != ONSET_SUBTYPE:
                continue
            if _int(row.get("source_code", "")) != ONSET_SOURCE:
                continue
            bar = _int(row.get("available_bar_index", ""))
            high = _float(row.get("primary_value", ""))
            low = _float(row.get("reference_value", ""))
            if bar is None or high is None or low is None or high <= low:
                continue
            by_bar.setdefault(bar, Onset(bar=bar, high=high, low=low))
    return tuple(by_bar[bar] for bar in sorted(by_bar))


def bundle_label(model_dir: Path) -> str:
    """Builds a stable label from the model bundle's parent directories."""
    year = model_dir.parent.name
    instrument = model_dir.parent.parent.name
    if year.isdigit():
        return f"{instrument}-{year}"
    return str(model_dir)


def load_bundle(model_dir: Path) -> Bundle:
    """Loads one model bundle."""
    readings = model_dir / "readings.csv"
    events = model_dir / "events.csv"
    if not readings.is_file() or not events.is_file():
        raise FileNotFoundError(f"{model_dir} is not a model bundle")
    return Bundle(
        label=bundle_label(model_dir),
        bars=parse_readings(readings),
        onsets=parse_onsets(events),
    )


def _next_onset_limit(
    positions: list[int], index: int, position: int, window: int
) -> int:
    if index + 1 < len(positions):
        return min(position + window, positions[index + 1] - 1)
    return position + window


def reconstruct(
    bundle: Bundle,
    window: int,
    horizons: tuple[int, ...],
    *,
    supersede: bool = True,
) -> list[Signal]:
    """Reconstructs the first close-beyond-range breakout for every onset.

    `supersede` is true for the onset events: an unfinished onset is dropped
    when the next onset arrives. It is false for the placebo range, where every
    non-coiling window completion is an independent candidate and the
    non-overlapping sample handles overlap.
    """
    if window <= 0:
        raise ValueError("window must be positive")
    if not horizons:
        raise ValueError("horizons must not be empty")
    by_index = {bar.index: position for position, bar in enumerate(bundle.bars)}
    events: list[tuple[int, Onset]] = []
    for onset in bundle.onsets:
        position = by_index.get(onset.bar)
        if position is not None:
            events.append((position, onset))
    events.sort(key=lambda item: item[0])
    positions = [position for position, _ in events]
    signals: list[Signal] = []
    for index, (position, onset) in enumerate(events):
        limit = (
            _next_onset_limit(positions, index, position, window)
            if supersede
            else position + window
        )
        limit = min(limit, len(bundle.bars) - 1)
        if limit <= position:
            continue
        break_position: int | None = None
        direction = 0
        for candidate in range(position + 1, limit + 1):
            bar = bundle.bars[candidate]
            if bar.close > onset.high:
                break_position = candidate
                direction = 1
                break
            if bar.close < onset.low:
                break_position = candidate
                direction = -1
                break
        if break_position is None:
            continue
        entry_position = break_position + 1
        if entry_position >= len(bundle.bars):
            continue
        entry = bundle.bars[entry_position].open
        if entry <= 0.0:
            continue
        returns: dict[int, float] = {}
        for horizon in horizons:
            end = break_position + horizon
            if end < len(bundle.bars):
                returns[horizon] = (
                    direction
                    * (bundle.bars[end].close - entry)
                    / entry
                    * 10_000.0
                )
        signals.append(
            Signal(
                bundle=bundle.label,
                onset_bar=onset.bar,
                break_bar=bundle.bars[break_position].index,
                direction=direction,
                entry=entry,
                returns=returns,
            )
        )
    return signals


def non_overlapping(signals: list[Signal], gap: int = CLUSTER_GAP_BARS) -> list[Signal]:
    """Keeps the first signal of each cluster separated by at least `gap` bars."""
    selected: list[Signal] = []
    last = -gap - 1
    for signal in sorted(signals, key=lambda item: item.break_bar):
        if signal.break_bar - last >= gap:
            selected.append(signal)
            last = signal.break_bar
    return selected


def values(signals: list[Signal], horizon: int) -> list[float]:
    return [signal.returns[horizon] for signal in signals if horizon in signal.returns]


def stats(signals: list[Signal], horizon: int) -> dict[str, object]:
    """Mean, median, hit rate, and 95% interval for one horizon."""
    sample = values(signals, horizon)
    if not sample:
        return {"count": 0}
    mean = math.fsum(sample) / len(sample)
    stdev = statistics.stdev(sample) if len(sample) > 1 else 0.0
    stderr = stdev / math.sqrt(len(sample)) if len(sample) > 1 else 0.0
    return {
        "count": len(sample),
        "mean_bps": round(mean, 4),
        "median_bps": round(statistics.median(sample), 4),
        "mean_gross_bps": round(mean, 4),
        "stdev_bps": round(stdev, 4),
        "stderr_bps": round(stderr, 4),
        "ci95_bps": [
            round(mean - Z95 * stderr, 4),
            round(mean + Z95 * stderr, 4),
        ],
        "lcb_bps": round(mean - Z95 * stderr, 4),
        "ucb_bps": round(mean + Z95 * stderr, 4),
        "hit_rate": round(sum(1 for value in sample if value > 0.0) / len(sample), 5),
    }


def side_stats(signals: list[Signal], horizon: int) -> dict[str, object]:
    return {
        "long": stats([signal for signal in signals if signal.direction > 0], horizon),
        "short": stats([signal for signal in signals if signal.direction < 0], horizon),
        "pooled": stats(signals, horizon),
    }


def baseline_means(bundle: Bundle, horizons: tuple[int, ...]) -> dict[str, dict[int, float]]:
    """Unconditional signed forward returns for every bar, by side."""
    result: dict[str, dict[int, float]] = {"long": {}, "short": {}}
    bars = bundle.bars
    for horizon in horizons:
        for side, sign in (("long", 1.0), ("short", -1.0)):
            sample: list[float] = []
            for position in range(len(bars) - 1):
                end = position + horizon
                if end >= len(bars):
                    continue
                entry = bars[position + 1].open
                if entry <= 0.0:
                    continue
                sample.append(
                    sign * (bars[end].close - entry) / entry * 10_000.0
                )
            if sample:
                result[side][horizon] = math.fsum(sample) / len(sample)
    return result


def _weighted_baseline(
    baseline: dict[str, dict[int, float]],
    signals: list[Signal],
    horizon: int,
) -> float | None:
    long_count = sum(1 for signal in signals if signal.direction > 0)
    short_count = sum(1 for signal in signals if signal.direction < 0)
    total = long_count + short_count
    if total == 0:
        return None
    long_mean = baseline["long"].get(horizon)
    short_mean = baseline["short"].get(horizon)
    if long_mean is None or short_mean is None:
        return None
    return (long_count * long_mean + short_count * short_mean) / total


def shifted_onsets(bundle: Bundle, bars: int = 5) -> tuple[Onset, ...]:
    """Builds the back-lag onset: the same rule one bar earlier."""
    by_index = {bar.index: position for position, bar in enumerate(bundle.bars)}
    shifted: list[Onset] = []
    for onset in bundle.onsets:
        position = by_index.get(onset.bar)
        if position is None or position < bars:
            continue
        window = bundle.bars[position - bars : position]
        high = max(bar.high for bar in window)
        low = min(bar.low for bar in window)
        if high > low:
            shifted.append(Onset(bar=bundle.bars[position - 1].index, high=high, low=low))
    return tuple(shifted)


def placebo_onsets(bundle: Bundle, bars: int = 5) -> tuple[Onset, ...]:
    """Every non-coiling 5-bar window completion, as the matched baseline."""
    result: list[Onset] = []
    for position in range(bars - 1, len(bundle.bars)):
        window = bundle.bars[position - bars + 1 : position + 1]
        if all(bar.context_code == COILING_CONTEXT_CODE for bar in window):
            continue
        high = max(bar.high for bar in window)
        low = min(bar.low for bar in window)
        if high > low:
            result.append(Onset(bar=bundle.bars[position].index, high=high, low=low))
    return tuple(result)


def random_p95(
    signals: list[Signal], horizon: int, seed: int, draws: int = RANDOM_DRAWS
) -> float | None:
    """95th percentile of seeded random-direction means on the same signals."""
    sample = [signal.returns[horizon] for signal in signals if horizon in signal.returns]
    if not sample:
        return None
    rng = random.Random(seed)
    means: list[float] = []
    for _ in range(draws):
        total = math.fsum(
            value if rng.random() < 0.5 else -value for value in sample
        )
        means.append(total / len(sample))
    means.sort()
    return means[int(0.95 * len(means)) - 1]


def compression_means(
    bundle: Bundle, horizons: tuple[int, ...]
) -> dict[str, dict[int, float]]:
    """Same-direction entry at the onset bar, before the breakout."""
    by_index = {bar.index: position for position, bar in enumerate(bundle.bars)}
    result: dict[str, dict[int, float]] = {"long": {}, "short": {}}
    for horizon in horizons:
        for side, sign in (("long", 1.0), ("short", -1.0)):
            sample: list[float] = []
            for onset in bundle.onsets:
                position = by_index.get(onset.bar)
                if position is None or position + 1 >= len(bundle.bars):
                    continue
                entry = bundle.bars[position + 1].open
                end = position + horizon
                if entry <= 0.0 or end >= len(bundle.bars):
                    continue
                sample.append(
                    sign * (bundle.bars[end].close - entry) / entry * 10_000.0
                )
            if sample:
                result[side][horizon] = math.fsum(sample) / len(sample)
    return result


def evaluate(
    bundles: list[Bundle],
    *,
    window: int,
    horizons: tuple[int, ...],
    primary: int,
    cost_bps: float,
    stress_bps: float,
    seed: int,
) -> dict[str, object]:
    """Evaluates the breakout rule, its controls, and the protocol gates."""
    if primary not in horizons:
        raise ValueError("primary horizon must be in horizons")
    full: list[Signal] = []
    non_overlap: list[Signal] = []
    baseline: dict[str, dict[int, float]] = {"long": {}, "short": {}}
    compression: dict[str, dict[int, float]] = {"long": {}, "short": {}}
    per_bundle: list[dict[str, object]] = []
    back_lag_signals: list[Signal] = []
    placebo_signals: list[Signal] = []
    for bundle in bundles:
        signals = reconstruct(bundle, window, horizons)
        selected = non_overlapping(signals)
        full.extend(signals)
        non_overlap.extend(selected)
        bundle_baseline = baseline_means(bundle, horizons)
        for side in ("long", "short"):
            for horizon, mean in bundle_baseline[side].items():
                baseline[side].setdefault(horizon, 0.0)
                baseline[side][horizon] += mean
        bundle_compression = compression_means(bundle, horizons)
        for side in ("long", "short"):
            for horizon, mean in bundle_compression[side].items():
                compression[side].setdefault(horizon, 0.0)
                compression[side][horizon] += mean
        shifted = Bundle(
            label=bundle.label, bars=bundle.bars, onsets=shifted_onsets(bundle)
        )
        back_lag_signals.extend(non_overlapping(reconstruct(shifted, window, horizons)))
        placebo = Bundle(
            label=bundle.label, bars=bundle.bars, onsets=placebo_onsets(bundle)
        )
        placebo_signals.extend(
            non_overlapping(reconstruct(placebo, window, horizons, supersede=False))
        )
        per_bundle.append(
            {
                "label": bundle.label,
                "bars": len(bundle.bars),
                "onsets": len(bundle.onsets),
                "signals": len(signals),
                "non_overlapping_signals": len(selected),
            }
        )
    bundles_count = max(1, len(bundles))
    for side in ("long", "short"):
        for horizon in list(baseline[side]):
            baseline[side][horizon] /= bundles_count
        for horizon in list(compression[side]):
            compression[side][horizon] /= bundles_count

    full_stats = side_stats(full, primary)
    non_stats = side_stats(non_overlap, primary)
    pooled = non_stats["pooled"]
    mean = float(pooled.get("mean_bps", 0.0))
    lcb = float(pooled.get("lcb_bps", 0.0))
    count = int(pooled.get("count", 0))

    baseline_mean = _weighted_baseline(baseline, non_overlap, primary)
    if baseline_mean is None:
        baseline_mean = 0.0
    fade_mean = -mean
    random_95 = random_p95(non_overlap, primary, seed)
    back_stats = stats(back_lag_signals, primary)
    back_mean = float(back_stats.get("mean_bps", 0.0))
    placebo_stats = stats(placebo_signals, primary)
    placebo_mean = float(placebo_stats.get("mean_bps", 0.0))
    compression_long = compression["long"].get(primary)
    compression_short = compression["short"].get(primary)
    compression_matched = _weighted_compression(
        compression_long, compression_short, non_overlap
    )

    years: dict[str, dict[str, object]] = {}
    for signal in non_overlap:
        year = signal.bundle
        if year not in years:
            years[year] = {"count": 0, "sum": 0.0}
        if primary in signal.returns:
            years[year]["count"] = int(years[year]["count"]) + 1
            years[year]["sum"] = float(years[year]["sum"]) + signal.returns[primary]
    per_year: dict[str, dict[str, object]] = {}
    positive_years = 0
    for year in sorted(years):
        count_year = int(years[year]["count"])
        mean_year = float(years[year]["sum"]) / count_year if count_year else 0.0
        if count_year and mean_year > 0.0:
            positive_years += 1
        per_year[year] = {
            "count": count_year,
            "mean_bps": round(mean_year, 4),
        }

    gates = {
        "G1_baseline": {
            "pass": mean > baseline_mean and lcb > baseline_mean,
            "signal_mean_bps": round(mean, 4),
            "signal_lcb_bps": round(lcb, 4),
            "baseline_mean_bps": round(baseline_mean, 4),
        },
        "G2_fade": {
            "pass": mean > fade_mean and lcb > fade_mean,
            "signal_mean_bps": round(mean, 4),
            "fade_mean_bps": round(fade_mean, 4),
        },
        "G3_random": {
            "pass": random_95 is not None and mean > random_95,
            "signal_mean_bps": round(mean, 4),
            "random_p95_bps": None if random_95 is None else round(random_95, 4),
        },
        "G4_back_lag": {
            "pass": back_mean <= 0.0
            or (mean - back_mean >= 5.0 and lcb - back_mean >= 5.0),
            "signal_mean_bps": round(mean, 4),
            "signal_lcb_bps": round(lcb, 4),
            "back_lag_mean_bps": round(back_mean, 4),
        },
        "G5_placebo": {
            "pass": mean > placebo_mean and lcb > placebo_mean,
            "signal_mean_bps": round(mean, 4),
            "signal_lcb_bps": round(lcb, 4),
            "placebo_mean_bps": round(placebo_mean, 4),
        },
        "G6_gross": {
            "pass": mean > 0.0 and lcb > 0.0,
            "signal_mean_bps": round(mean, 4),
            "signal_lcb_bps": round(lcb, 4),
        },
        "G7_cost": {
            "pass": lcb - stress_bps > 0.0,
            "signal_lcb_bps": round(lcb, 4),
            "base_cost_bps": cost_bps,
            "stress_cost_bps": stress_bps,
            "net_lcb_base_bps": round(lcb - cost_bps, 4),
            "net_lcb_stress_bps": round(lcb - stress_bps, 4),
        },
        "G8_stability": {
            "pass": positive_years >= 3,
            "positive_years": positive_years,
            "years": len(per_year),
        },
        "G9_sample": {
            "pass": count >= 200 and count >= 20,
            "signals": count,
            "clusters": count,
        },
    }
    overall = all(bool(gate["pass"]) for gate in gates.values())
    return {
        "schema": "quantrule.breakout-edge/v1",
        "timeframe_secs": TIMEFRAME_SECS,
        "window": window,
        "horizons": list(horizons),
        "primary_horizon": primary,
        "cost_bps": cost_bps,
        "stress_bps": stress_bps,
        "seed": seed,
        "cluster_gap_bars": CLUSTER_GAP_BARS,
        "bundles": per_bundle,
        "signals": {
            "full": side_stats(full, primary),
            "non_overlapping": non_stats,
            "horizons_full": {
                str(horizon): side_stats(full, horizon) for horizon in horizons
            },
            "horizons_non_overlapping": {
                str(horizon): side_stats(non_overlap, horizon) for horizon in horizons
            },
        },
        "baseline": {
            "long": {str(h): round(v, 4) for h, v in baseline["long"].items()},
            "short": {str(h): round(v, 4) for h, v in baseline["short"].items()},
            "matched_primary_bps": round(baseline_mean, 4),
        },
        "controls": {
            "fade_mean_bps": round(fade_mean, 4),
            "random_p95_bps": None if random_95 is None else round(random_95, 4),
            "back_lag": back_stats,
            "placebo": placebo_stats,
            "compression_only": {
                "long_primary_bps": None
                if compression_long is None
                else round(compression_long, 4),
                "short_primary_bps": None
                if compression_short is None
                else round(compression_short, 4),
                "matched_primary_bps": None
                if compression_matched is None
                else round(compression_matched, 4),
                "note": "entry at the onset bar, before the breakout; descriptive",
            },
        },
        "per_year": per_year,
        "gates": gates,
        "overall_pass": overall,
    }


def _weighted_compression(
    long_mean: float | None,
    short_mean: float | None,
    signals: list[Signal],
) -> float | None:
    long_count = sum(1 for signal in signals if signal.direction > 0)
    short_count = sum(1 for signal in signals if signal.direction < 0)
    total = long_count + short_count
    if total == 0 or long_mean is None or short_mean is None:
        return None
    return (long_count * long_mean + short_count * short_mean) / total


def render_markdown(result: dict[str, object]) -> str:
    lines = [
        "# Fresh-Onset 5m Breakout Raw Edge",
        "",
        "Rule: first strict close outside the fresh-onset range; entry next bar open; "
        "no stop, target, halt, or position state.",
        f"Window: {result['window']} bars. Primary horizon: {result['primary_horizon']} bars. "
        f"Costs: {result['cost_bps']} bps base, {result['stress_bps']} bps stress.",
        f"Seed: {result['seed']}. Cluster gap: {result['cluster_gap_bars']} bars.",
        "",
        "## Bundles",
        "",
        "| Bundle | Bars | Onsets | Signals | Non-overlapping |",
        "|---|---:|---:|---:|---:|",
    ]
    for bundle in result["bundles"]:
        lines.append(
            f"| {bundle['label']} | {bundle['bars']} | {bundle['onsets']} | "
            f"{bundle['signals']} | {bundle['non_overlapping_signals']} |"
        )
    lines += [
        "",
        "## Primary result",
        "",
        "| Sample | Side | Signals | Mean bps | Median bps | Hit rate | 95% LCB | Net at 6 bps | Net at 9 bps |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for sample_name, sample in (
        ("full", result["signals"]["full"]),
        ("non-overlapping", result["signals"]["non_overlapping"]),
    ):
        for side in ("pooled", "long", "short"):
            row = sample[side]
            if not row or not row.get("count"):
                continue
            mean = float(row["mean_bps"])
            lines.append(
                f"| {sample_name} | {side} | {row['count']} | {mean:.4f} | "
                f"{float(row['median_bps']):.4f} | {float(row['hit_rate']) * 100:.1f}% | "
                f"{float(row['lcb_bps']):.4f} | {mean - float(result['cost_bps']):.4f} | "
                f"{mean - float(result['stress_bps']):.4f} |"
            )
    lines += [
        "",
        "## Gates",
        "",
        "| Gate | Pass | Detail |",
        "|---|---|---|",
    ]
    for name, gate in result["gates"].items():
        detail = ", ".join(
            f"{key}={value}" for key, value in gate.items() if key != "pass"
        )
        lines.append(f"| {name} | {'yes' if gate['pass'] else 'no'} | {detail} |")
    lines += [
        "",
        f"Overall gate status: **{'pass' if result['overall_pass'] else 'fail'}**.",
        "",
        "## Per year",
        "",
        "| Year | Signals | Mean bps |",
        "|---|---:|---:|",
    ]
    for year, row in result["per_year"].items():
        lines.append(f"| {year} | {row['count']} | {row['mean_bps']:.4f} |")
    controls = result["controls"]
    lines += [
        "",
        "## Controls",
        "",
        "| Control | Mean / value |",
        "|---|---:|",
        f"| Baseline matched | {result['baseline']['matched_primary_bps']:.4f} bps |",
        f"| Fade | {controls['fade_mean_bps']:.4f} bps |",
        f"| Random direction p95 | {controls['random_p95_bps']} bps |",
        f"| Back-lag | {controls['back_lag'].get('mean_bps', '—')} bps |",
        f"| Placebo range | {controls['placebo'].get('mean_bps', '—')} bps |",
        f"| Compression-only long | {controls['compression_only']['long_primary_bps']} bps |",
        f"| Compression-only short | {controls['compression_only']['short_primary_bps']} bps |",
        "",
        "## Horizon detail (non-overlapping)",
        "",
        "| Bars | Side | Signals | Mean bps | LCB bps |",
        "|---:|---|---:|---:|---:|",
    ]
    for horizon, sample in result["signals"]["horizons_non_overlapping"].items():
        for side in ("pooled", "long", "short"):
            row = sample[side]
            if not row or not row.get("count"):
                continue
            lines.append(
                f"| {horizon} | {side} | {row['count']} | "
                f"{float(row['mean_bps']):.4f} | {float(row['lcb_bps']):.4f} |"
            )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", action="append", required=True, help="model bundle directory")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--window", type=int, default=DEFAULT_WINDOW)
    parser.add_argument("--horizons", default=",".join(str(h) for h in DEFAULT_HORIZONS))
    parser.add_argument("--primary", type=int, default=DEFAULT_PRIMARY)
    parser.add_argument("--cost-bps", type=float, default=DEFAULT_COST_BPS)
    parser.add_argument("--stress-bps", type=float, default=DEFAULT_STRESS_BPS)
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED)
    args = parser.parse_args()
    horizons = tuple(sorted({int(value) for value in args.horizons.split(",")}))
    if not horizons or min(horizons) <= 0:
        raise ValueError("horizons must be positive")
    bundles = [load_bundle(Path(path)) for path in args.model]
    bundles.sort(key=lambda bundle: bundle.label)
    result = evaluate(
        bundles,
        window=args.window,
        horizons=horizons,
        primary=args.primary,
        cost_bps=args.cost_bps,
        stress_bps=args.stress_bps,
        seed=args.seed,
    )
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "breakout_edge.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out / "breakout_edge.md").write_text(render_markdown(result), encoding="utf-8")
    pooled = result["signals"]["non_overlapping"]["pooled"]
    print(
        f"pooled: n={pooled.get('count')} mean={pooled.get('mean_bps')} bps "
        f"lcb={pooled.get('lcb_bps')} bps gates={'pass' if result['overall_pass'] else 'fail'}"
    )
    print(f"report: {out / 'breakout_edge.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
