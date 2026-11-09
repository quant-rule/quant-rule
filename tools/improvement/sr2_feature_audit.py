#!/usr/bin/env python3
"""Phase A of the SR2 feature programme: top-trade attribution and charts.

The tool reads the recorded SR2-A BTCUSD cells and writes a feature table, a
summary of the conditions present in the top trades, and 1,000-candle PNGs for
the largest trades. It is read-only over the backtest record. Statistics is
evaluation only; nothing here enters a model, a strategy, or a decision. The
attribution functions use the standard library; chart rendering requires
Pillow and imports it lazily.

Usage:
    python3 tools/improvement/sr2_feature_audit.py \
        --base data/improvement/2026-11-08/STRATEGY-2026-11-08-02/artifacts/A \
        --out data/improvement/2026-11-08/SR2-BTC-FEATURES \
        --top 5
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

CELLS = tuple(f"btc-{year}" for year in range(2018, 2023))
CANDLES = 1_000
RVOL_LOW = 1.4
ADX_WEAK = 25.0
EMA_COLORS = ((10, (240, 200, 50)), (50, (70, 190, 255)), (200, (235, 90, 200)))


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def ema(values: list[float], period: int) -> list[float | None]:
    if not values:
        return []
    seed = 2.0 / (period + 1.0)
    result: list[float | None] = []
    current = values[0]
    for index, value in enumerate(values):
        current = value if index == 0 else current + seed * (value - current)
        result.append(current if index >= period - 1 else None)
    return result


def window_bounds(index: int, length: int, count: int = CANDLES) -> tuple[int, int]:
    if length <= count:
        return 0, length
    start = max(0, index - count // 2)
    end = start + count
    if end > length:
        end = length
        start = end - count
    return start, end


def session(hour: int) -> str:
    if hour < 6:
        return "00-06"
    if hour < 12:
        return "06-12"
    if hour < 18:
        return "12-18"
    return "18-24"


def load_cell(cell: str, base: Path) -> dict[str, Any]:
    root = base / cell / "backtest"
    readings = {
        int(row["bar"]): row
        for row in read_csv(root / "model" / "readings.csv")
        if row["timeframe_secs"] == "3600"
    }
    decisions = {}
    for line in (root / "strategy" / "decisions.ndjson").open(encoding="utf-8"):
        record = json.loads(line)
        decisions[record["bar_index"]] = record
    events: dict[int, list[dict[str, str]]] = {}
    for row in read_csv(root / "model" / "events.csv"):
        if row["timeframe_secs"] != "3600":
            continue
        events.setdefault(int(row["available_bar_index"]), []).append(row)
    trades = read_csv(root / "portfolio" / "trades.csv")
    return {
        "readings": readings,
        "decisions": decisions,
        "events": events,
        "trades": trades,
    }


def feature_rows(cell: str, data: dict[str, Any]) -> list[dict[str, Any]]:
    rows = []
    for trade in data["trades"]:
        entry_bar = int(trade["entry_bar"])
        decision_bar = entry_bar - 1
        reading = data["readings"].get(decision_bar)
        decision = data["decisions"].get(decision_bar)
        if reading is None or decision is None or not decision.get("evidence"):
            continue
        evidence = decision["evidence"][0]
        if evidence.get("kind") != "event":
            continue
        direction = int(evidence["direction"])
        stack = int(reading["ema_stack"]) if reading["ema_stack"] else None
        adx = float(reading["adx"]) if reading["adx"] else None
        rvol = float(reading["rvol_ratio"]) if reading["rvol_ratio"] else None
        atr_ratio = float(reading["atr_ratio"]) if reading["atr_ratio"] else None
        rsi = float(reading["rsi"]) if reading["rsi"] else None
        timestamp = int(reading["timestamp"])
        hour = datetime.fromtimestamp(timestamp, tz=timezone.utc).hour
        rows.append(
            {
                "cell": cell,
                "trade_id": int(trade["trade_id"]),
                "side": trade["side"],
                "entry_date": datetime.fromtimestamp(
                    int(trade["entry_time"]), tz=timezone.utc
                ).date().isoformat(),
                "net_pnl": float(trade["net_pnl"]),
                "return_fraction": float(trade["return_fraction"]),
                "hold_bars": int(trade["hold_bars"]),
                "exit_reason": trade["exit_reason"],
                "event_direction": direction,
                "ema_stack": stack,
                "aligned": stack is not None and direction == stack,
                "adx": adx,
                "adx_weak": adx is not None and adx < ADX_WEAK,
                "adx_regime": int(reading["adx_regime"]) if reading["adx_regime"] else None,
                "rvol_ratio": rvol,
                "rvol_low": rvol is not None and rvol < RVOL_LOW,
                "atr_ratio": atr_ratio,
                "rsi": rsi,
                "hour": hour,
                "session": session(hour),
                "co_events": len(data["events"].get(decision_bar, [])),
                "decision_bar": decision_bar,
            }
        )
    return rows


def stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {"n": 0}
    net = [row["net_pnl"] for row in rows]
    profit = sum(value for value in net if value > 0.0)
    loss = sum(value for value in net if value < 0.0)
    return {
        "n": len(rows),
        "net_pnl": sum(net),
        "mean": sum(net) / len(net),
        "win_rate": sum(1 for value in net if value > 0.0) / len(net),
        "profit_factor": None if loss == 0.0 else profit / -loss,
    }


def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
    filters = {
        "all": lambda row: True,
        "aligned": lambda row: row["aligned"],
        "adx_weak": lambda row: row["adx_weak"],
        "rvol_low": lambda row: row["rvol_low"],
        "aligned_adx_weak": lambda row: row["aligned"] and row["adx_weak"],
        "aligned_rvol_low": lambda row: row["aligned"] and row["rvol_low"],
        "aligned_adx_weak_rvol_low": lambda row: row["aligned"]
        and row["adx_weak"]
        and row["rvol_low"],
    }
    summary: dict[str, Any] = {}
    for name, predicate in filters.items():
        selected = [row for row in rows if predicate(row)]
        summary[name] = stats(selected)
        summary[name]["by_year"] = {
            str(year): stats([row for row in selected if row["cell"] == f"btc-{year}"])
            for year in range(2018, 2023)
        }
    return summary


def draw_chart(
    cell: str, trade: dict[str, Any], data: dict[str, Any], rows: list[dict[str, Any]], out: Path
) -> Path:
    try:
        from PIL import Image, ImageDraw
    except ModuleNotFoundError as error:
        raise SystemExit(
            "chart rendering requires Pillow; install it with 'pip install Pillow'"
        ) from error

    bars = [data["readings"][bar] for bar in sorted(data["readings"])]
    index = next(
        (position for position, row in enumerate(bars) if int(row["bar"]) == trade["decision_bar"]),
        None,
    )
    if index is None:
        raise ValueError(f"decision bar not found for {cell} trade {trade['trade_id']}")
    start, end = window_bounds(index, len(bars))
    window = bars[start:end]
    closes = [float(row["close"]) for row in window]
    full_closes = [float(row["close"]) for row in bars]
    emas = {period: ema(full_closes, period) for period, _ in EMA_COLORS}
    highs = [float(row["high"]) for row in window]
    lows = [float(row["low"]) for row in window]
    volumes = [float(row["volume"]) for row in window]
    low_price = min(lows)
    high_price = max(highs)
    padding = (high_price - low_price) * 0.06 or 1.0
    low_price -= padding
    high_price += padding

    width, height = 2400, 1600
    left, right = 90, 40
    top, price_height, volume_height, strip_height = 70, 1000, 280, 150
    plot_width = width - left - right
    price_bottom = top + price_height
    volume_top = price_bottom + 30
    volume_bottom = volume_top + volume_height
    image = Image.new("RGB", (width, height), (16, 18, 24))
    draw = ImageDraw.Draw(image)
    step = plot_width / max(len(window), 1)

    def x(position: int) -> float:
        return left + (position + 0.5) * step

    def y(price: float) -> float:
        return top + (high_price - price) / (high_price - low_price) * price_height

    for position, row in enumerate(window):
        open_price = float(row["open"])
        close = float(row["close"])
        high = float(row["high"])
        low = float(row["low"])
        color = (60, 200, 130) if close >= open_price else (230, 90, 90)
        draw.line((x(position), y(high), x(position), y(low)), fill=color, width=1)
        body_top = y(max(open_price, close))
        body_bottom = y(min(open_price, close))
        if body_bottom - body_top < 1.0:
            body_bottom = body_top + 1.0
        draw.rectangle(
            (x(position) - max(step * 0.35, 0.7), body_top, x(position) + max(step * 0.35, 0.7), body_bottom),
            fill=color,
        )

    for period, color in EMA_COLORS:
        points = []
        for position, row in enumerate(window):
            absolute = start + position
            value = emas[period][absolute] if absolute < len(emas[period]) else None
            if value is not None:
                points.append((x(position), y(value)))
        if len(points) > 1:
            draw.line(points, fill=color, width=2)

    max_volume = max(volumes) if volumes else 1.0
    for position, volume in enumerate(volumes):
        bar_height = volume / max_volume * volume_height
        color = (60, 160, 110) if float(window[position]["close"]) >= float(window[position]["open"]) else (180, 70, 70)
        draw.rectangle(
            (x(position) - max(step * 0.3, 0.5), volume_bottom - bar_height, x(position) + max(step * 0.3, 0.5), volume_bottom),
            fill=color,
        )

    entry_bar = trade["decision_bar"] + 1
    entry_position = entry_bar - start
    exit_bar = entry_bar + trade["hold_bars"]
    exit_position = exit_bar - start
    if 0 <= entry_position < len(window):
        color = (80, 230, 140) if trade["side"] == "long" else (240, 90, 100)
        draw.polygon(
            (
                (x(entry_position), y(float(window[entry_position]["low"])) + 12),
                (x(entry_position) - 10, y(float(window[entry_position]["low"])) + 34),
                (x(entry_position) + 10, y(float(window[entry_position]["low"])) + 34),
            ),
            fill=color,
        )
        draw.text((x(entry_position) + 12, price_bottom - 40), f"{trade['side']} entry", fill=color)
    if 0 <= exit_position < len(window):
        color = (220, 220, 220)
        draw.line((x(exit_position), top, x(exit_position), volume_bottom), fill=(120, 120, 120), width=1)
        draw.text((x(exit_position) + 6, top + 4), f"exit {trade['exit_reason']}", fill=color)

    title = f"{cell} trade {trade['trade_id']}  {trade['side']}  {trade['entry_date']}  1h  {CANDLES} candles"
    draw.text((left, 22), title, fill=(235, 235, 235))
    draw.text(
        (left, price_bottom + 4),
        f"event {trade['event_direction']:+d}  stack {trade['ema_stack']}  ADX {trade['adx']:.1f}  RVOL {trade['rvol_ratio']:.2f}  ATRratio {trade['atr_ratio']:.2f}  session {trade['session']}  net {trade['net_pnl']:+.2f}",
        fill=(220, 220, 220),
    )
    draw.text((left, volume_bottom + 18), "EMA 10 / 50 / 200 (display only)   volume", fill=(180, 180, 180))
    draw.text(
        (left, volume_bottom + 48),
        f"aligned {str(trade['aligned']).lower()}  rvol_low {str(trade['rvol_low']).lower()}  adx_weak {str(trade['adx_weak']).lower()}  co-events {trade['co_events']}",
        fill=(180, 180, 180),
    )
    draw.line((left, volume_bottom + 10, width - right, volume_bottom + 10), fill=(70, 70, 70))
    path = out / f"{cell}-trade-{trade['trade_id']}.png"
    image.save(path)
    return path


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="SR2-A artifacts root")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--top", type=int, default=5, help="number of charts")
    args = parser.parse_args(argv[1:])
    base = Path(args.base).resolve()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    charts = out / "charts"
    charts.mkdir(exist_ok=True)

    rows: list[dict[str, Any]] = []
    cells: dict[str, dict[str, Any]] = {}
    for cell in CELLS:
        data = load_cell(cell, base)
        cells[cell] = data
        rows.extend(feature_rows(cell, data))
    rows.sort(key=lambda row: row["net_pnl"], reverse=True)

    fields = [
        "cell",
        "trade_id",
        "side",
        "entry_date",
        "net_pnl",
        "return_fraction",
        "hold_bars",
        "exit_reason",
        "event_direction",
        "ema_stack",
        "aligned",
        "adx",
        "adx_weak",
        "adx_regime",
        "rvol_ratio",
        "rvol_low",
        "atr_ratio",
        "rsi",
        "hour",
        "session",
        "co_events",
        "decision_bar",
    ]
    with (out / "features.csv").open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)

    top = rows[: args.top]
    for trade in top:
        draw_chart(trade["cell"], trade, cells[trade["cell"]], rows, charts)

    document = {
        "scope": "SR2-A BTCUSD 1h, 2018-2022",
        "candles_per_chart": CANDLES,
        "rvol_low_threshold": RVOL_LOW,
        "adx_weak_threshold": ADX_WEAK,
        "top_trades": top,
        "summary": summarize(rows),
        "charts": [f"charts/{trade['cell']}-trade-{trade['trade_id']}.png" for trade in top],
    }
    (out / "features.json").write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {out / 'features.json'} and {len(top)} charts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
