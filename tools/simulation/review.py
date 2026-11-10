#!/usr/bin/env python3
"""Summarize a simulation run's marks into a review table.

The review is descriptive. It compares the good and bad marks, shows the
captured part of the favorable excursion, and lists the context at the entry.
It does not fit a model, optimize a threshold, or claim an edge.

Usage:
    from review import build_review
    build_review(bundle, charts, marks, details, out_path, run_id, premise)
"""

from __future__ import annotations

from pathlib import Path
from statistics import mean
from typing import Any, Iterable

from bundle import Bundle, Event
from marks import count_by_quality

EMA_STACK = {-1: "bear", 0: "mixed", 1: "bull"}
RSI_ZONE = {-1: "oversold", 0: "middle", 1: "overbought"}


def _mean(values: Iterable[float | None]) -> str:
    present = [value for value in values if value is not None]
    if not present:
        return "—"
    return f"{mean(present):.2f}"


def _pct(value: float | None) -> str:
    return "—" if value is None else f"{value * 100:+.2f}%"


def _bar(bundle: Bundle, bar_id: int):
    return bundle.bars[bundle.index_of(bar_id)]


def _pairs(marks: list[dict[str, str]]) -> list[dict[str, Any]]:
    """Pair the first entry and the first exit for each chart/side/quality."""
    groups: dict[tuple[str, str, str], dict[str, list[dict[str, str]]]] = {}
    for row in marks:
        key = (row["chart"], row["side"], row["quality"])
        groups.setdefault(key, {"entry": [], "exit": []})[row["action"]].append(row)
    pairs = []
    for (chart, side, quality), group in sorted(groups.items()):
        entries = sorted(group["entry"], key=lambda row: int(row["bar"]))
        exits = sorted(group["exit"], key=lambda row: int(row["bar"]))
        if not entries or not exits:
            continue
        entry = entries[0]
        exit_row = next(
            (row for row in exits if int(row["bar"]) >= int(entry["bar"])), exits[0]
        )
        pairs.append(
            {
                "chart": chart,
                "side": side,
                "quality": quality,
                "entry_bar": int(entry["bar"]),
                "exit_bar": int(exit_row["bar"]),
            }
        )
    return pairs


def _context(bundle: Bundle, bar_id: int, catalog: dict[int, str]) -> dict[str, Any]:
    bar = _bar(bundle, bar_id)
    events = [event.label(catalog) for event in bundle.events_at(bar_id)]
    return {
        "ema_stack": EMA_STACK.get(bar.ema_stack, "—") if bar.ema_stack is not None else "—",
        "rsi_zone": RSI_ZONE.get(bar.rsi_zone, "—") if bar.rsi_zone is not None else "—",
        "rsi": bar.rsi,
        "atr_ratio": bar.atr_ratio,
        "adx": bar.adx,
        "close_position": bar.value("close_position"),
        "upper_wick": bar.value("upper_wick_ratio"),
        "lower_wick": bar.value("lower_wick_ratio"),
        "body_ratio": bar.value("body_ratio"),
        "events": events,
    }


def _trade_stats(bundle: Bundle, pair: dict[str, Any]) -> dict[str, Any]:
    entry = _bar(bundle, pair["entry_bar"])
    exit_bar = _bar(bundle, pair["exit_bar"])
    sign = 1.0 if pair["side"] == "long" else -1.0
    entry_price = entry.close
    exit_price = exit_bar.close
    return_pct = (exit_price - entry_price) / entry_price * sign if entry_price else None
    return {
        **pair,
        "entry_price": entry_price,
        "exit_price": exit_price,
        "return_pct": return_pct,
        "atr": entry.atr,
    }


def _detail_stats(details: list[dict[str, Any]]) -> dict[tuple[str, str, str], dict[str, Any]]:
    """Return the oracle detail keyed by chart/side/quality."""
    result = {}
    for row in details:
        key = (str(row["chart"]), str(row["side"]), str(row["quality"]))
        result[key] = row
    return result


def _table(
    rows: list[dict[str, Any]],
    details: dict[tuple[str, str, str], dict[str, Any]],
    bundle: Bundle,
    catalog: dict[int, str],
) -> str:
    lines = [
        "| Quality | Side | n | Mark return (close) | MFE to exit (ATR) | MAE to exit (ATR) | Captured (good only) |",
        "|---|---|---:|---:|---:|---:|---:|",
    ]
    groups: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for row in rows:
        groups.setdefault((row["quality"], row["side"]), []).append(row)
    for (quality, side), group in sorted(groups.items()):
        returns = [row["return_pct"] for row in group]
        detail_rows = [details[(row["chart"], side, quality)] for row in group if (row["chart"], side, quality) in details]
        mfes = [
            row.get("mfe_to_rule_atr", row.get("mfe_atr")) for row in detail_rows
        ]
        maes = [
            row.get("mae_to_rule_atr", row.get("mae_atr")) for row in detail_rows
        ]
        captured = []
        if quality == "good":
            for row in group:
                detail = details.get((row["chart"], side, quality))
                mfe = detail.get("mfe_to_rule_atr") if detail else None
                if mfe and row["atr"] and row["entry_price"]:
                    captured.append(
                        (row["return_pct"] or 0.0) / (mfe * row["atr"] / row["entry_price"])
                    )
        lines.append(
            f"| {quality} | {side} | {len(group)} | {_mean(returns)} | {_mean(mfes)} | {_mean(maes)} | {_pct(mean(captured)) if captured else '—'} |"
        )
    return "\n".join(lines)


def _context_table(
    rows: list[dict[str, Any]],
    bundle: Bundle,
    catalog: dict[int, str],
    focus_events: set[str],
) -> str:
    lines = [
        "| Quality | Side | EMA stack | RSI zone | Mean RSI | Mean ATR ratio | Mean ADX | Close pos | Upper wick | Lower wick | Body | Other events at entry |",
        "|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    groups: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for row in rows:
        groups.setdefault((row["quality"], row["side"]), []).append(row)
    for (quality, side), group in sorted(groups.items()):
        contexts = [_context(bundle, row["entry_bar"], catalog) for row in group]
        stacks: dict[str, int] = {}
        zones: dict[str, int] = {}
        events: dict[str, int] = {}
        for context in contexts:
            stacks[context["ema_stack"]] = stacks.get(context["ema_stack"], 0) + 1
            zones[context["rsi_zone"]] = zones.get(context["rsi_zone"], 0) + 1
            for event in context["events"]:
                if event in focus_events:
                    continue
                events[event] = events.get(event, 0) + 1
        stack_text = ", ".join(f"{key} {value}" for key, value in sorted(stacks.items()))
        zone_text = ", ".join(f"{key} {value}" for key, value in sorted(zones.items()))
        top_events = sorted(events.items(), key=lambda item: (-item[1], item[0]))[:6]
        event_text = ", ".join(f"{key} {value}" for key, value in top_events) or "—"
        lines.append(
            f"| {quality} | {side} | {stack_text} | {zone_text} | {_mean([row['rsi'] for row in contexts])} | "
            f"{_mean([row['atr_ratio'] for row in contexts])} | {_mean([row['adx'] for row in contexts])} | "
            f"{_mean([row['close_position'] for row in contexts])} | "
            f"{_mean([row['upper_wick'] for row in contexts])} | "
            f"{_mean([row['lower_wick'] for row in contexts])} | "
            f"{_mean([row['body_ratio'] for row in contexts])} | {event_text} |"
        )
    return "\n".join(lines)


def build_review(
    bundle: Bundle,
    charts: list[dict[str, Any]],
    marks: list[dict[str, str]],
    details: list[dict[str, Any]],
    out_path: Path,
    run_id: str,
    premise: str,
    catalog: dict[int, str],
    focus_events: set[str] | None = None,
) -> None:
    """Write ``review.md`` for one run."""
    rows = [_trade_stats(bundle, pair) for pair in _pairs(marks)]
    detail_by_key = _detail_stats(details)
    counts = count_by_quality(marks)
    lines = [
        f"# Review — {run_id}",
        "",
        f"- Premise: {premise}",
        "- Model: `M1.3`",
        "- Claim type: hindsight labels; no edge claim",
        "",
        "## Mark counts",
        "",
        "| Quality | Side | Marks |",
        "|---|---|---:|",
    ]
    for (quality, side), count in sorted(counts.items()):
        lines.append(f"| {quality} | {side} | {count} |")
    lines += [
        "",
        "## Round trips",
        "",
        _table(rows, detail_by_key, bundle, catalog),
        "",
        "## Context at the entry",
        "",
        _context_table(rows, bundle, catalog, focus_events or set()),
        "",
        "## Candidate rule",
        "",
        "- Good marks share: (fill after the visual review)",
        "- Bad marks share: (fill after the visual review)",
        "- Candidate entry condition: (at most two conditions)",
        "- Candidate exit:",
        "- Rejected candidate:",
        "",
        "## Limits",
        "",
        "- The marks use future bars by design. They are labels, not signals.",
        "- The good/bad split is the oracle's forward-path criterion, not a market prediction; the context table is the only part that can discriminate before the exit.",
        "- The mark return is close-to-close at the marked bars, not a fill price.",
        "- The oracle is a heuristic; the AI visual review is subjective.",
        "- A sample under 30 non-overlapping windows is underpowered.",
        "- The check window (2020–2022) is separate and is not marked here.",
        "",
    ]
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(lines), encoding="utf-8")
