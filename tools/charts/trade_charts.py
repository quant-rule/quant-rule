#!/usr/bin/env python3
"""Render deterministic 15m trade charts from a recorded backtest bundle.

The tool reads the model readings and portfolio trades that the shell already
wrote, selects up to 1,000 candles around each trade entry, and draws the chart
with volume, EMA 10/50/200, entry/exit markers, and stop/target lines when the
execution bundle records them. It is an evaluation and review tool: it reads a
recorded result and never computes a model, strategy, or decision.

Usage:
    python3 tools/charts/trade_charts.py \
        --base data/improvement/2026-11-09/SR9-15M-STRUCTURE/artifacts/BTCUSD \
        --run-glob '*/breakonly-9bps/long' \
        --out data/improvement/2026-11-09/SR9-15M-STRUCTURE/charts \
        --theme light
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

try:
    from PIL import Image, ImageDraw, ImageFont
except ModuleNotFoundError:  # pragma: no cover - exercised by the skip test
    Image = ImageDraw = ImageFont = None  # type: ignore[assignment]

MAX_CANDLES = 1_000
DEFAULT_TIMEFRAME = "15m"
TF_SECONDS = {"15m": 900, "1h": 3_600}


@dataclass(frozen=True)
class Bar:
    bar: int
    timestamp: int
    open: float
    high: float
    low: float
    close: float
    volume: float


@dataclass(frozen=True)
class Trade:
    trade_id: int
    side: str
    entry_bar: int
    entry_time: int
    entry_price: float
    exit_bar: int
    exit_time: int
    exit_price: float
    net_pnl: float
    return_fraction: float
    hold_bars: int
    exit_reason: str


@dataclass
class Run:
    path: Path
    label: str
    bars: list[Bar]
    trades: list[Trade]
    orders: list[dict[str, str]]


@dataclass(frozen=True)
class Theme:
    background: str
    panel: str
    text: str
    muted: str
    grid: str
    up: str
    down: str
    ema10: str
    ema50: str
    ema200: str
    entry_long: str
    entry_short: str
    stop: str
    target: str
    annotation: str


LIGHT = Theme(
    background="#f7f8fa",
    panel="#ffffff",
    text="#1f2933",
    muted="#667085",
    grid="#d9dee5",
    up="#0a8f5b",
    down="#c0392b",
    ema10="#d97706",
    ema50="#2563eb",
    ema200="#7c3aed",
    entry_long="#047857",
    entry_short="#b91c1c",
    stop="#dc2626",
    target="#059669",
    annotation="#111827",
)
DARK = Theme(
    background="#0f141a",
    panel="#161d26",
    text="#f3f4f6",
    muted="#9ca3af",
    grid="#2b3542",
    up="#34d399",
    down="#f87171",
    ema10="#fbbf24",
    ema50="#60a5fa",
    ema200="#c084fc",
    entry_long="#34d399",
    entry_short="#f87171",
    stop="#fb7185",
    target="#4ade80",
    annotation="#f9fafb",
)


def read_csv(path: Path) -> list[dict[str, str]]:
    """Read a CSV file with a header."""
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def load_run(path: Path, timeframe: str) -> Run:
    """Load one backtest run's readings, trades, and optional orders."""
    seconds = TF_SECONDS[timeframe]
    model_root = path / "backtest" / "model"
    readings_path = model_root / "readings.csv"
    trades_path = path / "backtest" / "portfolio" / "trades.csv"
    orders_path = path / "backtest" / "execution" / "orders.csv"
    if not readings_path.is_file():
        raise ValueError(f"missing readings: {readings_path}")
    if not trades_path.is_file():
        raise ValueError(f"missing trades: {trades_path}")
    bars: list[Bar] = []
    for row in read_csv(readings_path):
        if row["timeframe_secs"] != str(seconds):
            continue
        bars.append(
            Bar(
                bar=int(row["bar"]),
                timestamp=int(row["timestamp"]),
                open=float(row["open"]),
                high=float(row["high"]),
                low=float(row["low"]),
                close=float(row["close"]),
                volume=float(row["volume"]),
            )
        )
    bars.sort(key=lambda bar: bar.timestamp)
    if not bars:
        raise ValueError(f"no {timeframe} readings in {readings_path}")
    trades = [
        Trade(
            trade_id=int(row["trade_id"]),
            side=row["side"],
            entry_bar=int(row["entry_bar"]),
            entry_time=int(row["entry_time"]),
            entry_price=float(row["entry_price"]),
            exit_bar=int(row["exit_bar"]),
            exit_time=int(row["exit_time"]),
            exit_price=float(row["exit_price"]),
            net_pnl=float(row["net_pnl"]),
            return_fraction=float(row["return_fraction"]),
            hold_bars=int(row["hold_bars"]),
            exit_reason=row["exit_reason"],
        )
        for row in read_csv(trades_path)
    ]
    orders = read_csv(orders_path) if orders_path.is_file() else []
    label = next((part for part in reversed(path.parts) if part.isdigit() and len(part) == 4), path.name)
    return Run(path=path, label=label, bars=bars, trades=trades, orders=orders)


def load_runs(base: Path, run_glob: str, timeframe: str) -> list[Run]:
    """Load every run matching a glob, ordered by path."""
    paths = sorted(path for path in base.glob(run_glob) if path.is_dir())
    if not paths:
        raise ValueError(f"no runs matched {run_glob!r} under {base}")
    runs = [load_run(path, timeframe) for path in paths]
    runs.sort(key=lambda run: (run.bars[0].timestamp, str(run.path)))
    return runs


def ema(values: list[float], period: int) -> list[float | None]:
    """Compute a causal EMA seeded with the first close, as the chart display."""
    if not values:
        return []
    if period <= 0:
        raise ValueError("EMA period must be positive")
    seed = 2.0 / (period + 1.0)
    current = values[0]
    result: list[float | None] = []
    for index, value in enumerate(values):
        current = value if index == 0 else current + seed * (value - current)
        result.append(current if index >= period - 1 else None)
    return result


def window_bounds(entry_index: int, length: int, maximum: int = MAX_CANDLES) -> tuple[int, int]:
    """Return a window of at most ``maximum`` bars that contains the entry."""
    if maximum < 1:
        raise ValueError("maximum must be positive")
    if length <= maximum:
        return 0, length
    start = max(0, entry_index - maximum // 2)
    end = start + maximum
    if end > length:
        end = length
        start = end - maximum
    return start, end


def find_stop_target(run: Run, trade: Trade) -> tuple[float | None, float | None]:
    """Find the recorded stop and target trigger prices for one trade."""
    stop = target = None
    for order in run.orders:
        try:
            submitted = int(order["submitted_bar"])
        except (KeyError, ValueError):
            continue
        if not (trade.entry_bar <= submitted <= trade.exit_bar):
            continue
        reason = order.get("reason", "")
        try:
            price = float(order["trigger_price"])
        except (KeyError, ValueError):
            continue
        if reason == "stop_loss" and stop is None:
            stop = price
        elif reason == "take_profit" and target is None:
            target = price
    return stop, target


def format_price(value: float) -> str:
    """Format a price with thousands separators and useful precision."""
    if abs(value) >= 10:
        return f"{value:,.2f}"
    return f"{value:,.4f}"


def format_pct(value: float) -> str:
    """Format a return fraction as a signed percentage."""
    return f"{value * 100:+.2f}%"


def format_pnl(value: float) -> str:
    """Format a PnL with a currency sign and a plus for gains."""
    sign = "+" if value >= 0 else "-"
    return f"{sign}${abs(value):,.2f}"


def trade_actions(trade: Trade) -> tuple[str, str]:
    """Return the entry and exit actions for a trade."""
    return ("BUY", "SELL") if trade.side == "long" else ("SELL", "BUY")


def hex_rgb(value: str) -> tuple[int, int, int]:
    """Convert ``#rrggbb`` to an RGB tuple."""
    value = value.lstrip("#")
    return tuple(int(value[index : index + 2], 16) for index in (0, 2, 4))  # type: ignore[return-value]


def format_time(timestamp: int, with_date: bool = True) -> str:
    """Format a UTC timestamp."""
    moment = datetime.fromtimestamp(timestamp, tz=timezone.utc)
    return moment.strftime("%Y-%m-%d %H:%M UTC" if with_date else "%m-%d %H:%M")


def font(size: int, bold: bool = False) -> Any:
    """Load a readable font, falling back to Pillow's default."""
    candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    ]
    if ImageFont is None:
        raise RuntimeError("Pillow is required to render charts")
    for candidate in candidates:
        if Path(candidate).is_file():
            return ImageFont.truetype(candidate, size)
    return ImageFont.load_default()


def dashed_line(draw: Any, start: tuple[float, float], end: tuple[float, float], fill: str, width: int = 2, dash: int = 10, gap: int = 8) -> None:
    """Draw a dashed line."""
    x0, y0 = start
    x1, y1 = end
    length = math.hypot(x1 - x0, y1 - y0)
    if length == 0:
        return
    dx, dy = (x1 - x0) / length, (y1 - y0) / length
    position = 0.0
    while position < length:
        segment = min(dash, length - position)
        draw.line(
            (
                x0 + dx * position,
                y0 + dy * position,
                x0 + dx * (position + segment),
                y0 + dy * (position + segment),
            ),
            fill=fill,
            width=width,
        )
        position += dash + gap


def draw_label(draw: Any, xy: tuple[float, float], text: str, fill: str, background: str, text_font: Any, padding: int = 8) -> None:
    """Draw a text label with a readable background."""
    left, top, right, bottom = draw.textbbox((0, 0), text, font=text_font)
    width = right - left
    height = bottom - top
    x, y = xy
    draw.rounded_rectangle(
        (x - padding, y - padding, x + width + padding, y + height + padding * 2),
        radius=6,
        fill=background,
    )
    draw.text((x, y), text, fill=fill, font=text_font)


def draw_chart(
    run: Run,
    trade: Trade,
    out: Path,
    theme: Theme,
    title: str,
    strategy: str,
    timeframe: str,
    maximum: int,
) -> dict[str, Any]:
    """Draw one chart and return its manifest record."""
    if Image is None:
        raise RuntimeError("Pillow is required to render charts; install it with pip install Pillow")
    bar_index = {bar.bar: index for index, bar in enumerate(run.bars)}
    if trade.entry_bar not in bar_index:
        raise ValueError(f"entry bar {trade.entry_bar} is not in the readings")
    entry_index = bar_index[trade.entry_bar]
    start, end = window_bounds(entry_index, len(run.bars), maximum)
    window = run.bars[start:end]
    closes = [bar.close for bar in run.bars]
    emas = {period: ema(closes, period) for period in (10, 50, 200)}
    stop, target = find_stop_target(run, trade)

    width, height = 2_400, 1_600
    left, right = 110, 240
    top, price_bottom = 200, 1_160
    volume_top, volume_bottom = 1_215, 1_425
    plot_width = width - left - right
    price_low = min(bar.low for bar in window)
    price_high = max(bar.high for bar in window)
    if stop is not None:
        price_low = min(price_low, stop)
        price_high = max(price_high, stop)
    if target is not None:
        price_low = min(price_low, target)
        price_high = max(price_high, target)
    padding = (price_high - price_low) * 0.06 or 1.0
    price_low -= padding
    price_high += padding
    max_volume = max((bar.volume for bar in window), default=1.0) or 1.0

    image = Image.new("RGB", (width, height), theme.background)
    step = plot_width / max(len(window), 1)

    def x(position: int) -> float:
        return left + (position + 0.5) * step

    def y(price: float) -> float:
        return top + (price_high - price) / (price_high - price_low) * (price_bottom - top)

    entry_x = x(entry_index - start)
    exit_position = bar_index.get(trade.exit_bar, start) - start
    exit_x = x(exit_position) if 0 <= exit_position < len(window) else entry_x
    if exit_x > entry_x:
        overlay = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        tint = hex_rgb(theme.entry_long if trade.side == "long" else theme.entry_short)
        ImageDraw.Draw(overlay).rectangle((entry_x, top, exit_x, volume_bottom), fill=(*tint, 20))
        image = Image.alpha_composite(image.convert("RGBA"), overlay).convert("RGB")
    draw = ImageDraw.Draw(image)

    # Price grid and right-side labels.
    for tick in range(7):
        price = price_low + (price_high - price_low) * tick / 6
        yy = y(price)
        draw.line((left, yy, width - right, yy), fill=theme.grid, width=1)
        draw.text((width - right + 16, yy - 11), format_price(price), fill=theme.muted, font=font(20))

    # Time grid and bottom labels.
    for tick in range(9):
        position = round((len(window) - 1) * tick / 8)
        xx = x(position)
        draw.line((xx, top, xx, volume_bottom), fill=theme.grid, width=1)
        label = format_time(window[position].timestamp)
        draw.text((xx - 86, volume_bottom + 18), label, fill=theme.muted, font=font(18))

    # Candles and volume.
    body_half = max(step * 0.32, 0.8)
    for position, bar in enumerate(window):
        color = theme.up if bar.close >= bar.open else theme.down
        draw.line((x(position), y(bar.high), x(position), y(bar.low)), fill=color, width=max(1, int(step * 0.12)))
        body_top = y(max(bar.open, bar.close))
        body_bottom = y(min(bar.open, bar.close))
        if body_bottom - body_top < 1.0:
            body_bottom = body_top + 1.0
        draw.rectangle((x(position) - body_half, body_top, x(position) + body_half, body_bottom), fill=color)
        volume_height = bar.volume / max_volume * (volume_bottom - volume_top)
        draw.rectangle(
            (x(position) - body_half, volume_bottom - volume_height, x(position) + body_half, volume_bottom),
            fill=theme.up if bar.close >= bar.open else theme.down,
        )

    # EMA lines.
    ema_colors = {10: theme.ema10, 50: theme.ema50, 200: theme.ema200}
    for period, color in ema_colors.items():
        points = []
        for position, bar in enumerate(window):
            absolute = start + position
            value = emas[period][absolute] if absolute < len(emas[period]) else None
            if value is not None:
                points.append((x(position), y(value)))
        if len(points) > 1:
            draw.line(points, fill=color, width=4, joint="curve")

    # Stop and target lines.
    for value, color, label in ((stop, theme.stop, "stop"), (target, theme.target, "target")):
        if value is None:
            continue
        draw.line((entry_x, y(value), exit_x, y(value)), fill=color, width=3)
        draw.text((entry_x + 8, y(value) - 24), f"{label} {format_price(value)}", fill=color, font=font(18, bold=True))

    # Entry and exit markers. The exit is the opposite action: a long sells
    # out, a short buys back. Both markers are drawn so the chart never shows
    # only half of the round trip.
    entry_bar = window[entry_index - start]
    entry_action, exit_action = trade_actions(trade)
    entry_color = theme.entry_long if trade.side == "long" else theme.entry_short
    marker_y = y(entry_bar.low) + 18 if trade.side == "long" else y(entry_bar.high) - 18
    direction = 1 if trade.side == "long" else -1
    draw.polygon(
        (
            (entry_x, marker_y),
            (entry_x - 14, marker_y + direction * 24),
            (entry_x + 14, marker_y + direction * 24),
        ),
        fill=entry_color,
    )
    dashed_line(draw, (entry_x, top), (entry_x, volume_bottom), entry_color, width=2)
    draw_label(
        draw,
        (entry_x + 18, top + 16),
        f"ENTRY {entry_action} ({trade.side})  {format_time(trade.entry_time)}  @ {format_price(trade.entry_price)}",
        theme.annotation,
        theme.panel,
        font(22, bold=True),
    )
    if 0 <= exit_position < len(window):
        exit_bar = window[exit_position]
        exit_color = theme.entry_short if trade.side == "long" else theme.entry_long
        if trade.side == "long":
            exit_marker_y = max(top + 30, y(exit_bar.high) - 18)
            exit_points = (
                (exit_x, exit_marker_y - 24),
                (exit_x - 14, exit_marker_y),
                (exit_x + 14, exit_marker_y),
            )
        else:
            exit_marker_y = min(price_bottom - 30, y(exit_bar.low) + 18)
            exit_points = (
                (exit_x, exit_marker_y + 24),
                (exit_x - 14, exit_marker_y),
                (exit_x + 14, exit_marker_y),
            )
        draw.polygon(exit_points, fill=exit_color)
        dashed_line(draw, (exit_x, top), (exit_x, volume_bottom), exit_color, width=2)
        exit_text = f"EXIT {exit_action} ({trade.exit_reason})  {format_time(trade.exit_time)}  @ {format_price(trade.exit_price)}"
        exit_font = font(22, bold=True)
        exit_width = draw.textlength(exit_text, font=exit_font)
        if exit_x + 18 + exit_width + 16 <= width - right:
            exit_label_x = exit_x + 18
        else:
            exit_label_x = max(left, exit_x - exit_width - 34)
        draw_label(
            draw,
            (exit_label_x, top + 96),
            exit_text,
            theme.annotation,
            theme.panel,
            exit_font,
        )

    # Header and summary.
    draw.text((left, 34), f"{title} — trade {trade.trade_id}", fill=theme.text, font=font(52, bold=True))
    draw.text(
        (left, 102),
        f"{strategy} · {timeframe} · {format_time(window[0].timestamp)} → {format_time(window[-1].timestamp)} · {len(window):,} candles",
        fill=theme.muted,
        font=font(26),
    )
    summary = [
        f"Net {format_pnl(trade.net_pnl)}",
        f"Return {format_pct(trade.return_fraction)}",
        f"Hold {trade.hold_bars} bars",
        f"Exit {trade.exit_reason}",
    ]
    x_cursor = width - right
    for index, text in enumerate(summary):
        text_font = font(30 if index == 0 else 22, bold=index == 0)
        width_text = draw.textlength(text, font=text_font)
        draw.text(
            (x_cursor - width_text, 44 + index * 46),
            text,
            fill=theme.text if index == 0 else theme.muted,
            font=text_font,
        )

    # Legend and footer.
    legend_y = 164
    for offset, (period, color) in enumerate(ema_colors.items()):
        last_value = next((value for value in reversed(emas[period][:end]) if value is not None), None)
        legend_x = left + offset * 360
        draw.line((legend_x, legend_y, legend_x + 42, legend_y), fill=color, width=4)
        suffix = f"  {format_price(last_value)}" if last_value is not None else ""
        draw.text((legend_x + 52, legend_y - 13), f"EMA {period}{suffix}", fill=color, font=font(20, bold=True))
    draw.text((left + 1_080, legend_y - 13), "Volume", fill=theme.muted, font=font(20))
    draw.line((left, volume_bottom + 58, width - right, volume_bottom + 58), fill=theme.grid, width=1)
    draw.text(
        (left, volume_bottom + 74),
        "Evaluation chart only · source: recorded backtest readings · max 1,000 candles · UTC",
        fill=theme.muted,
        font=font(18),
    )

    out.parent.mkdir(parents=True, exist_ok=True)
    image.save(out, format="PNG", optimize=False, compress_level=6)
    digest = hashlib.sha256(out.read_bytes()).hexdigest()
    return {
        "file": out.name,
        "sha256": digest,
        "run": run.label,
        "trade_id": trade.trade_id,
        "side": trade.side,
        "entry_time": format_time(trade.entry_time),
        "exit_time": format_time(trade.exit_time),
        "entry_price": trade.entry_price,
        "exit_price": trade.exit_price,
        "net_pnl": trade.net_pnl,
        "return_fraction": trade.return_fraction,
        "hold_bars": trade.hold_bars,
        "exit_reason": trade.exit_reason,
        "window_start": format_time(window[0].timestamp),
        "window_end": format_time(window[-1].timestamp),
        "candles": len(window),
        "width": width,
        "height": height,
    }


def render(
    base: Path,
    out: Path,
    run_glob: str,
    timeframe: str,
    maximum: int,
    theme_name: str,
    title: str,
    strategy: str,
) -> list[dict[str, Any]]:
    """Render every trade in every matched run."""
    if maximum > MAX_CANDLES:
        raise ValueError(f"maximum candles cannot exceed {MAX_CANDLES}")
    if maximum < 1:
        raise ValueError("maximum candles must be positive")
    theme = LIGHT if theme_name == "light" else DARK
    runs = load_runs(base, run_glob, timeframe)
    charts = out / "charts"
    charts.mkdir(parents=True, exist_ok=True)
    records: list[dict[str, Any]] = []
    for run in runs:
        for trade in run.trades:
            filename = f"{title.split()[0].lower()}-{timeframe}-{run.label}-trade-{trade.trade_id}.png"
            record = draw_chart(
                run,
                trade,
                charts / filename,
                theme,
                title,
                strategy,
                timeframe,
                maximum,
            )
            records.append(record)
    records.sort(key=lambda record: (record["entry_time"], record["run"], record["trade_id"]))
    manifest = {
        "schema": "quantrule.trade-charts/v1",
        "base": str(base),
        "run_glob": run_glob,
        "timeframe": timeframe,
        "max_candles": maximum,
        "theme": theme_name,
        "title": title,
        "strategy": strategy,
        "charts": records,
    }
    (charts / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    with (charts / "index.csv").open("w", encoding="utf-8", newline="") as handle:
        fields = [
            "file",
            "run",
            "trade_id",
            "side",
            "entry_time",
            "exit_time",
            "entry_price",
            "exit_price",
            "net_pnl",
            "return_fraction",
            "hold_bars",
            "exit_reason",
            "window_start",
            "window_end",
            "candles",
            "sha256",
        ]
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for record in records:
            writer.writerow({field: record[field] for field in fields})
    return records


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--run-glob", default="*/breakonly-9bps/long")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--timeframe", default=DEFAULT_TIMEFRAME, choices=sorted(TF_SECONDS))
    parser.add_argument("--max-candles", type=int, default=MAX_CANDLES)
    parser.add_argument("--theme", default="light", choices=("light", "dark"))
    parser.add_argument("--title", default="BTCUSD 15m")
    parser.add_argument("--strategy", default="BTC break-only long control · 9 bps")
    args = parser.parse_args(argv[1:])
    try:
        records = render(
            args.base,
            args.out,
            args.run_glob,
            args.timeframe,
            args.max_candles,
            args.theme,
            args.title,
            args.strategy,
        )
    except (ValueError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"wrote {len(records)} charts to {args.out / 'charts'}")
    print(f"manifest: {args.out / 'charts' / 'manifest.json'}")
    print(f"index: {args.out / 'charts' / 'index.csv'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
