#!/usr/bin/env python3
"""Deterministic hindsight labeling for one simulation window.

The oracle proposes the best good and the most tempting bad entry on each side
under the pre-registered criterion. It uses future bars by design: a simulation
run is a hindsight label, not a causal signal. The AI reviews the proposal; the
human corrects it. The oracle never writes a strategy decision.

Usage:
    from oracle import OracleSpec, oracle_for_window
    marks, details = oracle_for_window(bundle, window, chart_name, spec, run_id)
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from bundle import Bar, Bundle
from window_select import Window


@dataclass(frozen=True)
class OracleSpec:
    """The pre-registered hindsight criterion for one run."""

    horizon_bars: int = 60
    target_atr: float = 2.0
    stop_atr: float = 1.0
    bad_stop_atr: float = 1.5
    bad_target_atr: float = 1.0
    entry_offset_max: int = 2

    def validate(self) -> None:
        """Reject an impossible criterion."""
        if self.horizon_bars < 2:
            raise ValueError("horizon_bars must be at least 2")
        if not 0 < self.stop_atr < self.target_atr:
            raise ValueError("require 0 < stop_atr < target_atr")
        if not 0 < self.bad_target_atr < self.bad_stop_atr:
            raise ValueError("require 0 < bad_target_atr < bad_stop_atr")
        if self.entry_offset_max < 0:
            raise ValueError("entry_offset_max must be non-negative")


@dataclass(frozen=True)
class PathResult:
    """The forward path of one candidate entry."""

    side: str
    entry_index: int
    entry_bar: int
    entry_price: float
    atr: float
    mfe_atr: float
    mae_atr: float
    mfe_index: int
    mae_index: int
    good_target_index: int | None
    good_stop_index: int | None
    bad_stop_index: int | None
    bad_target_index: int | None
    good: bool
    bad: bool
    perfect_exit_index: int | None
    rule_exit_index: int | None
    mfe_before_bad_stop: float
    mfe_to_rule_atr: float
    mae_to_rule_atr: float

    @property
    def reward_risk(self) -> float:
        """A simple ranking score; larger is a cleaner good mark."""
        return self.mfe_atr / max(self.mae_atr, 0.1)

    @property
    def entry_quality(self) -> str:
        """Return ``good``, ``bad``, or an empty string when neither."""
        if self.good:
            return "good"
        if self.bad:
            return "bad"
        return ""


def _forward_path(bundle: Bundle, entry_index: int, side: str, spec: OracleSpec) -> PathResult | None:
    """Evaluate one entry with the good and bad criteria."""
    entry = bundle.bars[entry_index]
    atr = entry.atr
    if atr is None or atr <= 0:
        return None
    entry_price = entry.close
    end = min(entry_index + spec.horizon_bars, len(bundle.bars) - 1)
    if end <= entry_index:
        return None

    best_fav = 0.0
    best_fav_index = entry_index
    worst_adv = 0.0
    worst_adv_index = entry_index
    good_target: int | None = None
    good_stop: int | None = None
    bad_stop: int | None = None
    bad_target: int | None = None
    pre_bad_stop_best = 0.0
    pre_bad_stop_index: int | None = None

    for index in range(entry_index + 1, end + 1):
        bar = bundle.bars[index]
        if side == "long":
            favorable = bar.high - entry_price
            adverse = entry_price - bar.low
        else:
            favorable = entry_price - bar.low
            adverse = bar.high - entry_price
        favorable_atr = favorable / atr
        adverse_atr = adverse / atr

        if bad_stop is None and favorable_atr > pre_bad_stop_best:
            pre_bad_stop_best = favorable_atr
            pre_bad_stop_index = index
        if favorable_atr > best_fav:
            best_fav = favorable_atr
            best_fav_index = index
        if adverse_atr > worst_adv:
            worst_adv = adverse_atr
            worst_adv_index = index
        if good_target is None and favorable_atr >= spec.target_atr:
            good_target = index
        if good_stop is None and adverse_atr >= spec.stop_atr:
            good_stop = index
        if bad_target is None and favorable_atr >= spec.bad_target_atr:
            bad_target = index
        if bad_stop is None and adverse_atr >= spec.bad_stop_atr:
            bad_stop = index

    good = good_target is not None and (good_stop is None or good_target < good_stop)
    bad = bad_stop is not None and (bad_target is None or bad_stop < bad_target)
    if good:
        perfect = best_fav_index
        rule = good_target
    elif bad:
        perfect = pre_bad_stop_index
        rule = bad_stop
    else:
        perfect = None
        rule = None

    mfe_to_rule = 0.0
    mae_to_rule = 0.0
    if rule is not None:
        for index in range(entry_index + 1, rule + 1):
            bar = bundle.bars[index]
            if side == "long":
                favorable = (bar.high - entry_price) / atr
                adverse = (entry_price - bar.low) / atr
            else:
                favorable = (entry_price - bar.low) / atr
                adverse = (bar.high - entry_price) / atr
            mfe_to_rule = max(mfe_to_rule, favorable)
            mae_to_rule = max(mae_to_rule, adverse)

    return PathResult(
        side=side,
        entry_index=entry_index,
        entry_bar=entry.bar,
        entry_price=entry_price,
        atr=atr,
        mfe_atr=best_fav,
        mae_atr=worst_adv,
        mfe_index=best_fav_index,
        mae_index=worst_adv_index,
        good_target_index=good_target,
        good_stop_index=good_stop,
        bad_stop_index=bad_stop,
        bad_target_index=bad_target,
        good=good,
        bad=bad,
        perfect_exit_index=perfect,
        rule_exit_index=rule,
        mfe_before_bad_stop=pre_bad_stop_best,
        mfe_to_rule_atr=mfe_to_rule,
        mae_to_rule_atr=mae_to_rule,
    )


def _entry_candidates(window: Window, bundle: Bundle, spec: OracleSpec) -> list[int]:
    """Return the entry indices the oracle may choose from one window."""
    base = window.candidate_index
    candidates = []
    for offset in range(spec.entry_offset_max + 1):
        index = base + offset
        if window.start_index <= index < window.end_index and index < len(bundle.bars) - 1:
            candidates.append(index)
    return candidates


def _select_good(results: list[PathResult]) -> PathResult | None:
    """Pick the cleanest good path: reward/risk first, then earliest."""
    good = [result for result in results if result.good]
    if not good:
        return None
    good.sort(key=lambda result: (-result.reward_risk, result.entry_index))
    return good[0]


def _select_bad(results: list[PathResult]) -> PathResult | None:
    """Pick the earliest tempting failure."""
    bad = [result for result in results if result.bad]
    if not bad:
        return None
    bad.sort(key=lambda result: (result.entry_index, -result.mae_atr))
    return bad[0]


def _mark_row(
    chart: str,
    bar: int,
    action: str,
    quality: str,
    side: str,
    note: str,
) -> dict[str, str]:
    """Build one mark row; the timestamp is filled by validation."""
    return {
        "chart": chart,
        "bar": str(bar),
        "marked_time_utc": "",
        "action": action,
        "quality": quality,
        "side": side,
        "note": note[:500],
    }


def _detail_row(
    chart: str,
    window: Window,
    result: PathResult,
    bundle: Bundle,
    quality: str,
    run_id: str,
) -> dict[str, Any]:
    """Build one detail row for the review sidecar."""
    return {
        "chart": chart,
        "run_id": run_id,
        "window_id": window.window_id,
        "side": result.side,
        "quality": quality,
        "entry_bar": result.entry_bar,
        "entry_index": result.entry_index,
        "entry_price": result.entry_price,
        "atr": result.atr,
        "rule_exit_bar": bundle.bars[result.rule_exit_index].bar if result.rule_exit_index is not None else None,
        "perfect_exit_bar": bundle.bars[result.perfect_exit_index].bar if result.perfect_exit_index is not None else None,
        "mfe_atr": result.mfe_atr,
        "mae_atr": result.mae_atr,
        "mfe_to_rule_atr": result.mfe_to_rule_atr,
        "mae_to_rule_atr": result.mae_to_rule_atr,
        "mfe_before_bad_stop_atr": result.mfe_before_bad_stop,
        "good": result.good,
        "bad": result.bad,
    }


def oracle_for_window(
    bundle: Bundle,
    window: Window,
    chart: str,
    spec: OracleSpec,
    run_id: str,
) -> tuple[list[dict[str, str]], list[dict[str, Any]]]:
    """Return the proposed marks and detail rows for one window."""
    spec.validate()
    marks: list[dict[str, str]] = []
    details: list[dict[str, Any]] = []
    candidates = _entry_candidates(window, bundle, spec)
    if not candidates:
        return marks, details
    for side in ("long", "short"):
        results = [
            result
            for result in (_forward_path(bundle, index, side, spec) for index in candidates)
            if result is not None
        ]
        good = _select_good(results)
        bad = _select_bad(results)
        # The annotator identity is chart/bar/action/side, not quality. A good
        # and a bad mark cannot share an entry or exit bar on the same side.
        if good is not None and bad is not None:
            if bad.entry_bar == good.entry_bar or bad.rule_exit_index == good.rule_exit_index:
                bad = None
        for result, quality in ((good, "good"), (bad, "bad")):
            if result is None or result.rule_exit_index is None:
                continue
            perfect = (
                bundle.bars[result.perfect_exit_index].bar
                if result.perfect_exit_index is not None
                else "?"
            )
            note = (
                f"{run_id} {quality} {side} hint mfe={result.mfe_atr:.2f}atr "
                f"mae={result.mae_atr:.2f}atr perfect_exit={perfect}; "
                "hindsight label, not a signal"
            )
            marks.append(
                _mark_row(chart, result.entry_bar, "entry", quality, side, note)
            )
            exit_bar = bundle.bars[result.rule_exit_index].bar
            marks.append(
                _mark_row(
                    chart,
                    exit_bar,
                    "exit",
                    quality,
                    side,
                    f"{run_id} rule exit {quality} {side}; {note}",
                )
            )
            details.append(_detail_row(chart, window, result, bundle, quality, run_id))
    return marks, details
