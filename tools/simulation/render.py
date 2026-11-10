#!/usr/bin/env python3
"""Render clean simulation charts with hindsight marks.

The chart shows candles, volume, one optional indicator panel, the run's event
markers, and the marks. It never shows a backtest trade, a future shading, or a
second run's marks, so the labeler cannot anchor on an existing trade.

The output is deterministic: the same window, studies, and marks produce the
same PNG bytes. The manifest repeats the plot geometry so the annotator can map
a click to a bar.

Usage:
    from render import render_windows
    records = render_windows(bundle, windows, marks, out_dir, run_id, studies)
"""

from __future__ import annotations

import csv
import hashlib
import json
import math
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

try:
    from PIL import Image, ImageDraw, ImageFont
except ModuleNotFoundError:  # pragma: no cover - exercised by the skip test
    Image = ImageDraw = ImageFont = None  # type: ignore[assignment]

from bundle import Bundle, Bar, utc_from_seconds
from window_select import Window

WIDTH = 2400
HEIGHT = 1600
LEFT = 110
RIGHT = 240
TOP = 200
PRICE_BOTTOM = 980
VOLUME_TOP = 1020
VOLUME_BOTTOM = 1160
INDICATOR_TOP = 1200
INDICATOR_BOTTOM = 1440
DEFAULT_STUDIES = ("ema",)
STUDY_COLORS = {
    "ema_fast": "#d97706",
    "ema_mid": "#2563eb",
    "ema_slow": "#7c3aed",
    "rsi": "#7c3aed",
    "rvol": "#0a8f5b",
    "adx": "#c0392b",
}


class RenderError(Exception):
    """The chart cannot be rendered."""


@dataclass(frozen=True)
class Theme:
    """A fixed chart palette."""

    background: str = "#f7f8fa"
    panel: str = "#ffffff"
    text: str = "#1f2933"
    muted: str = "#667085"
    grid: str = "#d9dee5"
    up: str = "#0a8f5b"
    down: str = "#c0392b"
    annotation: str = "#111827"
    good_long: str = "#0a8f5b"
    good_short: str = "#c0392b"
    bad: str = "#667085"
    event: str = "#7c3aed"
    candidate: str = "#111827"


LIGHT = Theme()
DARK = Theme(
    background="#0f141a",
    panel="#161d26",
    text="#f3f4f6",
    muted="#9ca3af",
    grid="#2b3542",
    up="#34d399",
    down="#f87171",
    annotation="#f9fafb",
    good_long="#34d399",
    good_short="#f87171",
    bad="#9ca3af",
    event="#c084fc",
    candidate="#f9fafb",
)


def hex_rgb(value: str) -> tuple[int, int, int]:
    """Convert ``#rrggbb`` to an RGB tuple."""
    value = value.lstrip("#")
    return tuple(int(value[index : index + 2], 16) for index in (0, 2, 4))  # type: ignore[return-value]


def font(size: int, bold: bool = False) -> Any:
    """Load a readable font, falling back to Pillow's default."""
    if ImageFont is None:
        raise RenderError("Pillow is required to render charts; install it with pip install Pillow")
    candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
        if bold
        else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf"
        if bold
        else "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    ]
    for candidate in candidates:
        if Path(candidate).is_file():
            return ImageFont.truetype(candidate, size)
    return ImageFont.load_default()


def format_price(value: float) -> str:
    """Format a price with thousands separators and useful precision."""
    if abs(value) >= 10:
        return f"{value:,.2f}"
    return f"{value:,.4f}"


def format_time(timestamp: int, with_date: bool = True) -> str:
    """Format a UTC timestamp."""
    moment = datetime.fromtimestamp(timestamp, tz=timezone.utc)
    return moment.strftime("%Y-%m-%d %H:%M UTC" if with_date else "%m-%d %H:%M")


def dashed_line(
    draw: Any,
    start: tuple[float, float],
    end: tuple[float, float],
    fill: str,
    width: int = 2,
    dash: int = 10,
    gap: int = 8,
) -> None:
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


def draw_label(
    draw: Any,
    xy: tuple[float, float],
    text: str,
    fill: str,
    background: str,
    text_font: Any,
    padding: int = 8,
) -> None:
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


def _bar_map(bars: Iterable[Bar]) -> dict[int, Bar]:
    return {bar.bar: bar for bar in bars}


def _mark_style(mark: dict[str, str], theme: Theme) -> tuple[str, str, bool]:
    """Return colour, shape, and filled for one mark."""
    side = mark.get("side", "long")
    quality = mark.get("quality", "good")
    if quality == "bad":
        return theme.bad, "circle", False
    color = theme.good_long if side == "long" else theme.good_short
    return color, "circle" if side == "long" else "square", True


def _draw_mark(
    draw: Any,
    geometry: dict[str, float],
    bar: Bar,
    mark: dict[str, str],
    theme: Theme,
    step: float,
    bar_index: dict[int, int],
    window_start: int,
    label_offset: int = 0,
) -> None:
    """Draw one entry or exit marker, its vertical guide, and a letter label."""
    position = bar_index.get(bar.bar)
    if position is None:
        return
    x = geometry["left"] + (position - window_start + 0.5) * step
    color, shape, filled = _mark_style(mark, theme)
    action = mark.get("action", "entry")
    side = mark.get("side", "long")
    buy = (action == "entry" and side == "long") or (action == "exit" and side == "short")
    y = geometry["y"](bar.low) + 18 if buy else geometry["y"](bar.high) - 18
    y = max(geometry["top"] + 10, min(geometry["bottom"] - 10, y))
    radius = max(6.0, step * 0.55)
    dashed_line(draw, (x, geometry["top"]), (x, geometry["bottom"]), color, width=1, dash=4, gap=6)
    common = {"outline": color, "width": 3, "fill": color if filled else None}
    if shape == "square":
        draw.rectangle((x - radius, y - radius, x + radius, y + radius), **common)
    else:
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), **common)
    quality = "G" if mark.get("quality") == "good" else "B"
    label = f"{quality}{'E' if action == 'entry' else 'X'}"
    draw_label(
        draw,
        (x + radius + 4, y - radius + label_offset),
        label,
        color,
        theme.panel,
        font(16, bold=True),
        padding=3,
    )


def _indicator_values(bars: list[Bar], study: str) -> list[float | None]:
    """Return the selected indicator series."""
    if study == "rsi":
        return [bar.rsi for bar in bars]
    if study == "rvol":
        return [bar.rvol for bar in bars]
    if study == "adx":
        return [bar.adx for bar in bars]
    return []


def _draw_indicator(
    draw: Any,
    bars: list[Bar],
    study: str,
    geometry: dict[str, Any],
    theme: Theme,
) -> None:
    """Draw one indicator panel below volume."""
    values = _indicator_values(bars, study)
    if not values:
        return
    present = [value for value in values if value is not None]
    if not present:
        return
    low = min(present)
    high = max(present)
    if math.isclose(low, high):
        low -= 1.0
        high += 1.0
    padding = (high - low) * 0.12
    low -= padding
    high += padding

    def y(value: float) -> float:
        return INDICATOR_TOP + (high - value) / (high - low) * (INDICATOR_BOTTOM - INDICATOR_TOP)

    draw.line((LEFT, INDICATOR_BOTTOM, WIDTH - RIGHT, INDICATOR_BOTTOM), fill=theme.grid, width=1)
    for tick in range(3):
        value = low + (high - low) * tick / 2
        yy = y(value)
        draw.line((LEFT, yy, WIDTH - RIGHT, yy), fill=theme.grid, width=1)
        draw.text((WIDTH - RIGHT + 16, yy - 10), f"{value:.2f}", fill=theme.muted, font=font(16))
    if study == "rsi":
        for level in (30.0, 70.0):
            if low <= level <= high:
                dashed_line(draw, (LEFT, y(level)), (WIDTH - RIGHT, y(level)), theme.muted, width=1, dash=4, gap=6)
    elif study == "rvol":
        if low <= 1.0 <= high:
            dashed_line(draw, (LEFT, y(1.0)), (WIDTH - RIGHT, y(1.0)), theme.muted, width=1, dash=4, gap=6)
    elif study == "adx":
        if low <= 25.0 <= high:
            dashed_line(draw, (LEFT, y(25.0)), (WIDTH - RIGHT, y(25.0)), theme.muted, width=1, dash=4, gap=6)
    points = []
    for position, value in enumerate(values):
        if value is None:
            continue
        points.append((geometry["x"](position), y(value)))
    if len(points) > 1:
        draw.line(points, fill=STUDY_COLORS[study], width=3, joint="curve")
    draw.text((LEFT, INDICATOR_TOP - 26), study.upper(), fill=STUDY_COLORS[study], font=font(20, bold=True))


def render_window(
    bundle: Bundle,
    window: Window,
    chart_name: str,
    out_dir: Path,
    run_id: str,
    marks: list[dict[str, str]],
    catalog: dict[int, str],
    studies: tuple[str, ...] = DEFAULT_STUDIES,
    title: str = "BTCUSDT 15m",
    theme: Theme = LIGHT,
) -> dict[str, Any]:
    """Render one window and return its manifest record."""
    if Image is None:
        raise RenderError("Pillow is required to render charts; install it with pip install Pillow")
    bars = bundle.bars[window.start_index : window.end_index]
    if not bars:
        raise RenderError(f"window {window.window_id} has no bars")
    price_low = min(bar.low for bar in bars)
    price_high = max(bar.high for bar in bars)
    padding = (price_high - price_low) * 0.06 or 1.0
    price_low -= padding
    price_high += padding
    max_volume = max((bar.volume for bar in bars), default=1.0) or 1.0
    plot_width = WIDTH - LEFT - RIGHT
    step = plot_width / max(len(bars), 1)

    def x(position: int) -> float:
        return LEFT + (position + 0.5) * step

    def y(price: float) -> float:
        return TOP + (price_high - price) / (price_high - price_low) * (PRICE_BOTTOM - TOP)

    geometry = {
        "left": LEFT,
        "right": WIDTH - RIGHT,
        "top": TOP,
        "bottom": PRICE_BOTTOM,
        "x": x,
        "y": y,
    }
    bar_index = {bar.bar: index + window.start_index for index, bar in enumerate(bars)}
    bar_by_id = _bar_map(bars)
    image = Image.new("RGB", (WIDTH, HEIGHT), theme.background)
    draw = ImageDraw.Draw(image)

    # Price grid and right-side labels.
    for tick in range(7):
        price = price_low + (price_high - price_low) * tick / 6
        yy = y(price)
        draw.line((LEFT, yy, WIDTH - RIGHT, yy), fill=theme.grid, width=1)
        draw.text((WIDTH - RIGHT + 16, yy - 11), format_price(price), fill=theme.muted, font=font(20))

    # Time grid and bottom labels.
    for tick in range(9):
        position = round((len(bars) - 1) * tick / 8)
        xx = x(position)
        draw.line((xx, TOP, xx, INDICATOR_BOTTOM), fill=theme.grid, width=1)
        draw.text((xx - 86, INDICATOR_BOTTOM + 16), format_time(bars[position].timestamp), fill=theme.muted, font=font(18))

    # Candles and volume.
    body_half = max(step * 0.32, 0.8)
    for position, bar in enumerate(bars):
        color = theme.up if bar.close >= bar.open else theme.down
        draw.line((x(position), y(bar.high), x(position), y(bar.low)), fill=color, width=max(1, int(step * 0.12)))
        body_top = y(max(bar.open, bar.close))
        body_bottom = y(min(bar.open, bar.close))
        if body_bottom - body_top < 1.0:
            body_bottom = body_top + 1.0
        draw.rectangle((x(position) - body_half, body_top, x(position) + body_half, body_bottom), fill=color)
        volume_height = bar.volume / max_volume * (VOLUME_BOTTOM - VOLUME_TOP)
        draw.rectangle(
            (x(position) - body_half, VOLUME_BOTTOM - volume_height, x(position) + body_half, VOLUME_BOTTOM),
            fill=theme.up if bar.close >= bar.open else theme.down,
        )

    # EMA studies.
    if "ema" in studies:
        for period, color in ((20, STUDY_COLORS["ema_fast"]), (50, STUDY_COLORS["ema_mid"]), (200, STUDY_COLORS["ema_slow"])):
            key = {20: "ema_fast", 50: "ema_mid", 200: "ema_slow"}[period]
            points = []
            for position, bar in enumerate(bars):
                value = getattr(bar, key)
                if value is not None:
                    points.append((x(position), y(value)))
            if len(points) > 1:
                draw.line(points, fill=color, width=4, joint="curve")
            draw.line((LEFT + (period - 20) * 8, 164, LEFT + 42 + (period - 20) * 8, 164), fill=color, width=4)
            draw.text((LEFT + 52 + (period - 20) * 8, 151), f"EMA {period}", fill=color, font=font(20, bold=True))
    draw.text((LEFT + 1_230, 151), "Volume", fill=theme.muted, font=font(20))

    # Event markers for the selected events.
    event_font = font(18, bold=True)
    for event in window.events:
        bar = bar_by_id.get(event.available_bar)
        if bar is None:
            continue
        position = bar_index[bar.bar] - window.start_index
        xx = x(position)
        yy = max(TOP + 12, y(bar.high) - 18)
        draw.polygon(
            ((xx, yy), (xx - 9, yy - 16), (xx + 9, yy - 16)),
            fill=theme.event,
        )
        label = event.label(catalog)
        if len(window.events) <= 4:
            draw_label(draw, (xx + 10, yy - 34), label, theme.event, theme.panel, event_font, padding=4)

    # Candidate guide.
    candidate = bars[window.candidate_index - window.start_index]
    dashed_line(
        draw,
        (x(window.candidate_index - window.start_index), TOP),
        (x(window.candidate_index - window.start_index), INDICATOR_BOTTOM),
        theme.candidate,
        width=2,
        dash=8,
        gap=8,
    )
    draw_label(
        draw,
        (x(window.candidate_index - window.start_index) + 12, TOP + 10),
        f"candidate {format_time(candidate.timestamp)}",
        theme.candidate,
        theme.panel,
        event_font,
    )

    # Marks.
    for index, mark in enumerate(marks):
        try:
            bar_id = int(mark["bar"])
        except (KeyError, TypeError, ValueError):
            continue
        bar = bar_by_id.get(bar_id)
        if bar is None:
            continue
        _draw_mark(
            draw,
            geometry,
            bar,
            mark,
            theme,
            step,
            bar_index,
            window.start_index,
            label_offset=(index % 2) * 20,
        )

    # Header and footer.
    draw.text((LEFT, 34), f"{title} — {window.window_id}", fill=theme.text, font=font(50, bold=True))
    draw.text(
        (LEFT, 102),
        f"{run_id} · {format_time(bars[0].timestamp)} → {format_time(bars[-1].timestamp)} · "
        f"{len(bars):,} candles · candidate {format_time(candidate.timestamp)}",
        fill=theme.muted,
        font=font(24),
    )
    draw.text(
        (LEFT, HEIGHT - 78),
        "filled = good · hollow = bad · circle = long · square = short · "
        "below bar = buy · above bar = sell",
        fill=theme.muted,
        font=font(18),
    )
    draw.text(
        (LEFT, HEIGHT - 50),
        "Simulation chart · hindsight marks · not a signal · UTC",
        fill=theme.muted,
        font=font(18),
    )

    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / chart_name
    image.save(out_path, format="PNG", optimize=False, compress_level=6)
    digest = hashlib.sha256(out_path.read_bytes()).hexdigest()
    return {
        "file": chart_name,
        "sha256": digest,
        "run_id": run_id,
        "window_id": window.window_id,
        "candles": len(bars),
        "width": WIDTH,
        "height": HEIGHT,
        "timeframe_secs": bundle.timeframe_secs,
        "plot_left": LEFT,
        "plot_width": plot_width,
        "plot_right": WIDTH - RIGHT,
        "price_top": TOP,
        "price_bottom": PRICE_BOTTOM,
        "price_low": price_low,
        "price_high": price_high,
        "window_start_bar": bars[0].bar,
        "window_end_bar": bars[-1].bar,
        "start_time": format_time(bars[0].timestamp),
        "end_time": format_time(bars[-1].timestamp),
        "candidate_bar": candidate.bar,
        "candidate_time": format_time(candidate.timestamp),
        "studies": list(studies),
        "bars": [[bar.bar, bar.timestamp, bar.high, bar.low] for bar in bars],
    }


def render_windows(
    bundle: Bundle,
    windows_record: dict[str, Any],
    marks: list[dict[str, str]],
    out_dir: Path,
    run_id: str,
    catalog: dict[int, str],
    studies: tuple[str, ...] = DEFAULT_STUDIES,
    title: str = "BTCUSDT 15m",
    theme: Theme = LIGHT,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Render every selected window and write the chart manifest."""
    charts: list[dict[str, Any]] = []
    for record in windows_record["windows"]:
        # The renderer needs the event records for the markers; rebuild them
        # from the bundle by available bar and keep only the run's codes.
        codes = {int(event["code"]) for event in record["events"]}
        events = tuple(
            event for event in bundle.events_at(record["candidate_bar"]) if event.code in codes
        )
        window = Window(
            window_id=record["window_id"],
            start_index=record["start_index"],
            end_index=record["end_index"],
            candidate_index=record["candidate_index"],
            events=events,
        )
        chart_name = f"{run_id.lower()}-{window.window_id}.png"
        chart_marks = [mark for mark in marks if mark.get("chart") == chart_name]
        chart = render_window(
            bundle,
            window,
            chart_name,
            out_dir,
            run_id,
            chart_marks,
            catalog,
            studies,
            title,
            theme,
        )
        charts.append(chart)
    charts.sort(key=lambda chart: (chart["start_time"], chart["window_id"]))
    manifest = {
        "schema": "quantrule.simulation-charts/v1",
        "run_id": run_id,
        "title": title,
        "theme": "light" if theme is LIGHT else "dark",
        "studies": list(studies),
        "charts": charts,
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    with (out_dir / "index.csv").open("w", encoding="utf-8", newline="") as handle:
        fields = [
            "file",
            "run_id",
            "window_id",
            "candidate_time",
            "start_time",
            "end_time",
            "candles",
            "sha256",
        ]
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for chart in charts:
            writer.writerow({field: chart[field] for field in fields})
    return charts, manifest


def contact_sheet(
    charts_dir: Path,
    manifest: dict[str, Any],
    out_path: Path,
    columns: int = 3,
    rows: int = 4,
    thumb_width: int = 760,
) -> None:
    """Write a deterministic contact sheet of the first charts."""
    if Image is None:
        raise RenderError("Pillow is required to render charts; install it with pip install Pillow")
    charts = manifest["charts"][: columns * rows]
    if not charts:
        raise RenderError("the manifest has no charts")
    thumb_height = int(thumb_width * HEIGHT / WIDTH)
    label_height = 38
    sheet = Image.new(
        "RGB",
        (columns * thumb_width, rows * (thumb_height + label_height)),
        (247, 248, 250),
    )
    draw = ImageDraw.Draw(sheet)
    for index, chart in enumerate(charts):
        image = Image.open(charts_dir / chart["file"]).convert("RGB")
        image.thumbnail((thumb_width, thumb_height))
        column = index % columns
        row = index // columns
        x = column * thumb_width
        y = row * (thumb_height + label_height)
        sheet.paste(image, (x, y))
        draw.text(
            (x + 8, y + thumb_height + 6),
            f"{chart['window_id']} · {chart['candidate_time']}",
            fill=(31, 41, 51),
            font=font(20, bold=True),
        )
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, format="PNG", optimize=False, compress_level=6)
