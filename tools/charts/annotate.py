#!/usr/bin/env python3
"""Serve a local click-to-annotate page for recorded trade charts.

The page shows the existing PNG chart and writes annotations.csv directly. The
click is mapped to a bar by the chart manifest geometry; no pixel detection and
no hand-edited CSV are involved.

Usage:
    python3 tools/charts/annotate.py \
        --charts data/improvement/2026-11-09/BINANCE-BTCUSDT/charts \
        --out data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations-2018.csv
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
import threading
import webbrowser
from datetime import datetime, timezone
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlparse

ANNOTATIONS_HEADER = ["chart", "bar", "marked_time_utc", "action", "quality", "side", "note"]
ACTIONS = {"entry", "exit"}
QUALITIES = {"good", "bad", "neutral"}
SIDES = {"long", "short"}
UNDO_LIMIT = 100

CLASSES = [
    {"key": "1", "action": "entry", "quality": "good", "side": "long", "label": "Good long entry", "color": "#0a8f5b", "shape": "circle", "filled": True},
    {"key": "2", "action": "exit", "quality": "good", "side": "long", "label": "Good long exit", "color": "#c0392b", "shape": "circle", "filled": True},
    {"key": "3", "action": "entry", "quality": "bad", "side": "long", "label": "Bad long entry", "color": "#0a8f5b", "shape": "circle", "filled": False},
    {"key": "4", "action": "exit", "quality": "bad", "side": "long", "label": "Bad long exit", "color": "#c0392b", "shape": "circle", "filled": False},
    {"key": "5", "action": "entry", "quality": "good", "side": "short", "label": "Good short entry", "color": "#c0392b", "shape": "square", "filled": True},
    {"key": "6", "action": "exit", "quality": "good", "side": "short", "label": "Good short exit", "color": "#0a8f5b", "shape": "square", "filled": True},
    {"key": "7", "action": "entry", "quality": "bad", "side": "short", "label": "Bad short entry", "color": "#c0392b", "shape": "square", "filled": False},
    {"key": "8", "action": "exit", "quality": "bad", "side": "short", "label": "Bad short exit", "color": "#0a8f5b", "shape": "square", "filled": False},
]


def utc_from_seconds(seconds: int) -> str:
    """Format UTC seconds as an ISO timestamp."""
    return datetime.fromtimestamp(seconds, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_utc(value: str) -> int:
    """Parse an ISO UTC timestamp into seconds."""
    moment = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if moment.tzinfo is None:
        moment = moment.replace(tzinfo=timezone.utc)
    return int(moment.timestamp())


def chart_bars(record: dict[str, Any]) -> dict[int, list[Any]]:
    """Return bar -> [bar, timestamp, high, low]."""
    return {int(row[0]): row for row in record["bars"]}


def bar_from_x(record: dict[str, Any], x: float) -> int | None:
    """Map an image x coordinate to the bar at that candle."""
    left = float(record["plot_left"])
    width = float(record["plot_width"])
    if x < left or x > left + width:
        return None
    count = int(record["candles"])
    step = width / count
    index = int((x - left) / step)
    index = max(0, min(count - 1, index))
    return int(record["bars"][index][0])


def class_for(row: dict[str, Any]) -> dict[str, Any]:
    """Find the visual class for an annotation."""
    for item in CLASSES:
        if (
            row.get("action") == item["action"]
            and row.get("quality") == item["quality"]
            and row.get("side") == item["side"]
        ):
            return item
    return {
        "key": "?",
        "action": row.get("action", "entry"),
        "quality": row.get("quality", "neutral"),
        "side": row.get("side", "long"),
        "label": "Custom",
        "color": "#667085",
        "shape": "circle",
        "filled": False,
    }


def normalize_row(row: dict[str, str], manifest: dict[str, Any]) -> dict[str, str]:
    """Normalize an existing row, deriving bar from timestamp when needed."""
    normalized = {key: row.get(key, "") for key in ANNOTATIONS_HEADER}
    if not normalized["bar"]:
        chart = next(
            (item for item in manifest["charts"] if item["file"] == normalized["chart"]),
            None,
        )
        if chart is not None:
            target = parse_utc(normalized["marked_time_utc"])
            nearest = min(chart["bars"], key=lambda item: abs(int(item[1]) - target))
            if abs(int(nearest[1]) - target) <= int(chart["timeframe_secs"]) // 2:
                normalized["bar"] = str(nearest[0])
    return normalized


def read_annotations(path: Path, manifest: dict[str, Any]) -> list[dict[str, str]]:
    """Read and normalize the annotation CSV."""
    if not path.is_file():
        return []
    with path.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    return [normalize_row(row, manifest) for row in rows]


def write_annotations(path: Path, rows: list[dict[str, str]]) -> None:
    """Write annotations atomically with a stable order."""
    path.parent.mkdir(parents=True, exist_ok=True)
    ordered = sorted(
        rows,
        key=lambda row: (
            row.get("chart", ""),
            int(row["bar"]) if row.get("bar", "").isdigit() else 0,
            row.get("action", ""),
            row.get("side", ""),
        ),
    )
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=ANNOTATIONS_HEADER)
        writer.writeheader()
        for row in ordered:
            writer.writerow({key: row.get(key, "") for key in ANNOTATIONS_HEADER})
    temporary.replace(path)


def validate_annotation(value: dict[str, Any], manifest: dict[str, Any]) -> dict[str, str]:
    """Validate one annotation and derive its timestamp from the bar."""
    chart_name = str(value.get("chart", ""))
    chart = next((item for item in manifest["charts"] if item["file"] == chart_name), None)
    if chart is None:
        raise ValueError(f"unknown chart {chart_name!r}")
    try:
        bar = int(value["bar"])
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError("bar must be an integer") from error
    bars = chart_bars(chart)
    if bar not in bars:
        raise ValueError(f"bar {bar} is not in {chart_name}")
    action = str(value.get("action", ""))
    quality = str(value.get("quality", ""))
    side = str(value.get("side", ""))
    if action not in ACTIONS:
        raise ValueError(f"unknown action {action!r}")
    if quality not in QUALITIES:
        raise ValueError(f"unknown quality {quality!r}")
    if side not in SIDES:
        raise ValueError(f"unknown side {side!r}")
    note = str(value.get("note", "")).strip()
    if len(note) > 500:
        raise ValueError("note is too long")
    return {
        "chart": chart_name,
        "bar": str(bar),
        "marked_time_utc": utc_from_seconds(int(bars[bar][1])),
        "action": action,
        "quality": quality,
        "side": side,
        "note": note,
    }


def annotation_key(value: dict[str, Any]) -> tuple[str, str, str, str]:
    """Return the identity of an annotation."""
    return (
        str(value.get("chart", "")),
        str(value.get("bar", "")),
        str(value.get("action", "")),
        str(value.get("side", "")),
    )


def find_annotation(
    rows: list[dict[str, str]], key: tuple[str, str, str, str]
) -> dict[str, str] | None:
    """Find the annotation with the given identity."""
    return next((row for row in rows if annotation_key(row) == key), None)


def upsert(rows: list[dict[str, str]], row: dict[str, str]) -> list[dict[str, str]]:
    """Replace an annotation with the same chart/bar/action/side."""
    key = annotation_key(row)
    remaining = [item for item in rows if annotation_key(item) != key]
    return remaining + [row]


def delete_key(
    rows: list[dict[str, str]], key: tuple[str, str, str, str]
) -> list[dict[str, str]]:
    """Delete the annotation with the given identity."""
    return [item for item in rows if annotation_key(item) != key]


def delete(rows: list[dict[str, str]], value: dict[str, Any]) -> list[dict[str, str]]:
    """Delete an annotation by chart/bar/action/side."""
    return delete_key(rows, annotation_key(value))


def push_history(history: list[dict[str, Any]], entry: dict[str, Any]) -> None:
    """Record one reversible change, keeping the history bounded."""
    history.append(entry)
    if len(history) > UNDO_LIMIT:
        del history[0]


def record_save(
    history: list[dict[str, Any]],
    key: tuple[str, str, str, str],
    previous: dict[str, str] | None,
) -> None:
    """Record a save and the row it replaced, if any."""
    push_history(history, {"kind": "save", "key": key, "previous": previous})


def record_delete(history: list[dict[str, Any]], row: dict[str, str]) -> None:
    """Record a delete so the row can be restored."""
    push_history(history, {"kind": "delete", "row": row})


def undo_last(
    rows: list[dict[str, str]], history: list[dict[str, Any]]
) -> list[dict[str, str]]:
    """Reverse the most recent save or delete."""
    if not history:
        raise ValueError("nothing to undo")
    entry = history.pop()
    if entry["kind"] == "save":
        previous = entry["previous"]
        if previous is None:
            return delete_key(rows, entry["key"])
        return upsert(rows, previous)
    if entry["kind"] == "delete":
        return upsert(rows, entry["row"])
    raise ValueError(f"unknown undo entry {entry['kind']!r}")


HTML = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>QuantRule chart annotator</title>
<style>
:root { color-scheme: light; }
body { margin: 0; font-family: system-ui, -apple-system, Segoe UI, sans-serif; background: #eef1f5; color: #1f2933; }
header { display: flex; gap: 10px; align-items: center; padding: 10px 14px; background: #fff; border-bottom: 1px solid #d9dee5; flex-wrap: wrap; }
header h1 { font-size: 16px; margin: 0 12px 0 0; }
button, select { font: inherit; padding: 6px 9px; border: 1px solid #c7ced8; background: #fff; border-radius: 6px; cursor: pointer; }
button.active { outline: 2px solid #111827; outline-offset: 1px; }
button.mode { display: inline-flex; gap: 6px; align-items: center; }
.dot { display: inline-block; width: 12px; height: 12px; border-radius: 50%; border: 2px solid currentColor; }
.dot.square { border-radius: 2px; }
.mode-label { font-size: 13px; }
#wrap { position: relative; width: 100%; max-width: 1800px; margin: 12px auto; background: #fff; box-shadow: 0 1px 3px #0002; }
#chart { display: block; width: 100%; height: auto; user-select: none; }
#overlay { position: absolute; left: 0; top: 0; width: 100%; height: 100%; pointer-events: none; }
#status { padding: 4px 14px 8px; font-size: 13px; color: #475467; }
#list { max-width: 1800px; margin: 0 auto 30px; padding: 0 14px; }
#list table { width: 100%; border-collapse: collapse; background: #fff; font-size: 13px; }
#list th, #list td { text-align: left; padding: 5px 7px; border-bottom: 1px solid #e5e7eb; }
#list button { padding: 2px 6px; }
.help { font-size: 12px; color: #667085; margin-left: auto; }
</style>
</head>
<body>
<header>
  <h1>Chart annotator</h1>
  <button id="prev">←</button>
  <button id="next">→</button>
  <button id="undo" title="Undo the last annotation (Ctrl+Z)">Undo</button>
  <select id="chart-select"></select>
  <span id="trade-meta"></span>
  <span class="help">1–8 select marker · click candle to save · Ctrl+Z undo · ←/→ change chart</span>
</header>
<div id="modes" style="padding:10px 14px; display:flex; gap:6px; flex-wrap:wrap; background:#fff; border-bottom:1px solid #d9dee5;"></div>
<div id="wrap"><img id="chart" alt="chart"><svg id="overlay" viewBox="0 0 2400 1600" preserveAspectRatio="none"></svg></div>
<div id="status">Loading…</div>
<div id="list"><h2 style="font-size:15px;">Annotations</h2><table><thead><tr><th>Chart</th><th>Time</th><th>Action</th><th>Quality</th><th>Side</th><th>Note</th><th></th></tr></thead><tbody id="rows"></tbody></table></div>
<script>
const state = { manifest: null, annotations: [], index: 0, mode: null };
const $ = (id) => document.getElementById(id);
const NS = "http://www.w3.org/2000/svg";

function record() { return state.manifest.charts[state.index]; }
function step(r) { return r.plot_width / r.candles; }
function bars(r) { return new Map(r.bars.map((b) => [b[0], b])); }
function xForBar(r, bar) { return r.plot_left + (r.bars.findIndex((b) => b[0] === bar) + 0.5) * step(r); }
function yForPrice(r, price) { return r.price_top + (r.price_high - price) / (r.price_high - r.price_low) * (r.price_bottom - r.price_top); }
function barInfo(r, bar) { return bars(r).get(bar); }
function markerPoint(r, bar, action, side) {
  const b = barInfo(r, bar);
  if (!b) return null;
  const buy = (action === "entry" && side === "long") || (action === "exit" && side === "short");
  const y = buy ? yForPrice(r, b[3]) + 14 : yForPrice(r, b[2]) - 14;
  return { x: xForBar(r, bar), y: Math.max(r.price_top + 8, Math.min(r.price_bottom - 8, y)) };
}
function classFor(a) {
  return state.manifest.classes.find((c) => c.action === a.action && c.quality === a.quality && c.side === a.side)
    || { color: "#667085", shape: "circle", filled: false };
}
function svg(tag, attrs) { const node = document.createElementNS(NS, tag); for (const [k,v] of Object.entries(attrs)) node.setAttribute(k, v); return node; }
function drawMarker(g, p, cls, radius) {
  const common = { stroke: cls.color, "stroke-width": 3, fill: cls.filled ? cls.color : "none" };
  if (cls.shape === "square") g.appendChild(svg("rect", { x: p.x - radius, y: p.y - radius, width: radius*2, height: radius*2, rx: 2, ...common }));
  else g.appendChild(svg("circle", { cx: p.x, cy: p.y, r: radius, ...common }));
}
function render() {
  const r = record();
  $("chart").src = "/charts/" + encodeURIComponent(r.file) + "?v=" + Date.now();
  $("chart-select").value = String(state.index);
  $("trade-meta").textContent = `${r.run} · trade ${r.trade_id} · ${r.side} · ${r.entry_time} → ${r.exit_time}`;
  const overlay = $("overlay"); overlay.innerHTML = "";
  const g = svg("g", {});
  const barStep = step(r);
  const radius = Math.max(5, barStep * 0.48);
  for (const a of state.annotations.filter((x) => x.chart === r.file)) {
    const b = barInfo(r, parseInt(a.bar, 10));
    if (!b) continue;
    const p = markerPoint(r, parseInt(a.bar, 10), a.action, a.side);
    if (!p) continue;
    const cls = classFor(a);
    g.appendChild(svg("line", { x1: p.x, y1: r.price_top, x2: p.x, y2: r.volume_bottom || r.price_bottom, stroke: cls.color, "stroke-width": 1, "stroke-dasharray": "4 6", opacity: 0.45 }));
    drawMarker(g, p, cls, radius);
  }
  overlay.appendChild(g);
  renderModes();
  renderList();
  $("status").textContent = `Showing ${r.file} · ${r.candles} candles · ${state.annotations.filter((x) => x.chart === r.file).length} marks · click a candle to save the selected marker`;
}
function renderModes() {
  const root = $("modes"); root.innerHTML = "";
  for (const c of state.manifest.classes) {
    const btn = document.createElement("button");
    btn.className = "mode" + (state.mode && state.mode.key === c.key ? " active" : "");
    btn.innerHTML = `<span class="dot ${c.shape}" style="color:${c.color}; background:${c.filled ? c.color : "transparent"}"></span><span class="mode-label">${c.key} ${c.label}</span>`;
    btn.onclick = () => { state.mode = c; renderModes(); };
    root.appendChild(btn);
  }
}
function renderList() {
  const body = $("rows"); body.innerHTML = "";
  const chart = record();
  for (const a of state.annotations.filter((x) => x.chart === chart.file)) {
    const tr = document.createElement("tr");
    for (const value of [a.chart, a.marked_time_utc, a.action, a.quality, a.side, a.note]) {
      const td = document.createElement("td"); td.textContent = value; tr.appendChild(td);
    }
    const td = document.createElement("td");
    const del = document.createElement("button"); del.textContent = "×";
    del.onclick = async () => { await fetch("/api/annotations/delete", {method:"POST", headers:{"content-type":"application/json"}, body: JSON.stringify(a)}); await reload(); };
    td.appendChild(del); tr.appendChild(td); body.appendChild(tr);
  }
}
async function reload() {
  state.annotations = await (await fetch("/api/annotations")).json();
  render();
}
async function undo() {
  const response = await fetch("/api/annotations/undo", {method:"POST"});
  if (!response.ok) {
    let message = "Nothing to undo";
    try { message = (await response.json()).error || message; } catch (_) {}
    $("status").textContent = message;
    return;
  }
  state.annotations = await response.json();
  render();
  $("status").textContent = "Undid the last annotation · click a candle to save the selected marker";
}
async function save(bar) {
  if (!state.mode) return;
  const r = record();
  const payload = { chart: r.file, bar, action: state.mode.action, quality: state.mode.quality, side: state.mode.side, note: "" };
  const response = await fetch("/api/annotations", {method:"POST", headers:{"content-type":"application/json"}, body: JSON.stringify(payload)});
  if (!response.ok) { $("status").textContent = await response.text(); return; }
  await reload();
}
$("chart").addEventListener("click", (event) => {
  const r = record();
  const rect = $("chart").getBoundingClientRect();
  const x = (event.clientX - rect.left) * r.width / rect.width;
  const y = (event.clientY - rect.top) * r.height / rect.height;
  if (x < r.plot_left || x > r.plot_left + r.plot_width || y < r.price_top || y > r.price_bottom) return;
  const index = Math.max(0, Math.min(r.candles - 1, Math.floor((x - r.plot_left) / step(r))));
  save(r.bars[index][0]);
});
$("prev").onclick = () => { state.index = (state.index - 1 + state.manifest.charts.length) % state.manifest.charts.length; render(); };
$("next").onclick = () => { state.index = (state.index + 1) % state.manifest.charts.length; render(); };
$("undo").onclick = undo;
$("chart-select").onchange = (event) => { state.index = parseInt(event.target.value, 10); render(); };
document.addEventListener("keydown", (event) => {
  if (event.ctrlKey || event.metaKey) {
    if (!event.shiftKey && event.key.toLowerCase() === "z") {
      event.preventDefault();
      undo();
    }
    return;
  }
  if (event.key >= "1" && event.key <= "8") { state.mode = state.manifest.classes.find((c) => c.key === event.key); renderModes(); }
  if (event.key === "ArrowLeft") $("prev").click();
  if (event.key === "ArrowRight") $("next").click();
});
(async () => {
  state.manifest = await (await fetch("/api/manifest")).json();
  state.mode = state.manifest.classes[0];
  const select = $("chart-select");
  for (const [i, chart] of state.manifest.charts.entries()) {
    const option = document.createElement("option"); option.value = String(i); option.textContent = `${chart.run} trade ${chart.trade_id} · ${chart.side}`; select.appendChild(option);
  }
  await reload();
})();
</script>
</body>
</html>
"""


class AnnotatorHandler(BaseHTTPRequestHandler):
    """Serve the annotator and persist annotations."""

    manifest: dict[str, Any]
    charts_dir: Path
    out_path: Path
    rows: list[dict[str, str]]
    history: list[dict[str, Any]]
    lock: threading.Lock

    def log_message(self, format: str, *args: object) -> None:
        return

    def send_json(self, value: Any, status: HTTPStatus = HTTPStatus.OK) -> None:
        payload = json.dumps(value, sort_keys=True).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self) -> None:  # noqa: N802
        path = urlparse(self.path).path
        if path == "/":
            payload = HTML.encode("utf-8")
            self.send_response(HTTPStatus.OK)
            self.send_header("content-type", "text/html; charset=utf-8")
            self.send_header("content-length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        if path == "/api/manifest":
            self.send_json(self.manifest)
            return
        if path == "/api/annotations":
            with self.lock:
                rows = list(self.rows)
            self.send_json(rows)
            return
        if path.startswith("/charts/"):
            name = Path(unquote(path.removeprefix("/charts/"))).name
            record = next((item for item in self.manifest["charts"] if item["file"] == name), None)
            if record is None:
                self.send_error(HTTPStatus.NOT_FOUND)
                return
            payload = (self.charts_dir / name).read_bytes()
            self.send_response(HTTPStatus.OK)
            self.send_header("content-type", "image/png")
            self.send_header("content-length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        self.send_error(HTTPStatus.NOT_FOUND)

    def do_POST(self) -> None:  # noqa: N802
        path = urlparse(self.path).path
        if path not in {
            "/api/annotations",
            "/api/annotations/delete",
            "/api/annotations/undo",
        }:
            self.send_error(HTTPStatus.NOT_FOUND)
            return
        length = int(self.headers.get("content-length", "0"))
        try:
            value = {} if path.endswith("/undo") else json.loads(self.rfile.read(length) or b"{}")
        except ValueError as error:
            self.send_json({"error": str(error)}, HTTPStatus.BAD_REQUEST)
            return
        with self.lock:
            before_rows = list(self.rows)
            before_history = list(self.history)
            try:
                if path.endswith("/undo"):
                    if not self.history:
                        self.send_json({"error": "nothing to undo"}, HTTPStatus.CONFLICT)
                        return
                    self.rows[:] = undo_last(self.rows, self.history)
                elif path.endswith("/delete"):
                    removed = find_annotation(self.rows, annotation_key(value))
                    if removed is not None:
                        self.rows[:] = delete(self.rows, value)
                        record_delete(self.history, removed)
                else:
                    row = validate_annotation(value, self.manifest)
                    key = annotation_key(row)
                    previous = find_annotation(self.rows, key)
                    self.rows[:] = upsert(self.rows, row)
                    record_save(self.history, key, previous)
                write_annotations(self.out_path, self.rows)
            except ValueError as error:
                self.rows[:] = before_rows
                self.history[:] = before_history
                self.send_json({"error": str(error)}, HTTPStatus.BAD_REQUEST)
                return
            except OSError as error:
                self.rows[:] = before_rows
                self.history[:] = before_history
                self.send_json(
                    {"error": f"could not write annotations: {error}"},
                    HTTPStatus.INTERNAL_SERVER_ERROR,
                )
                return
            result = list(self.rows)
        self.send_json(result)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--charts", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--open", action="store_true", help="open the page in the default browser")
    args = parser.parse_args(argv[1:])
    manifest_path = args.charts / "manifest.json"
    if not manifest_path.is_file():
        print(f"error: {manifest_path} does not exist", file=sys.stderr)
        return 1
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    required = {"file", "candles", "plot_left", "plot_width", "price_top", "price_bottom", "price_low", "price_high", "bars", "timeframe_secs"}
    missing = required - set(manifest["charts"][0]) if manifest.get("charts") else required
    if missing:
        print(f"error: chart manifest is missing {sorted(missing)}", file=sys.stderr)
        return 1
    manifest["classes"] = CLASSES
    rows = read_annotations(args.out, manifest)
    handler = type(
        "BoundAnnotatorHandler",
        (AnnotatorHandler,),
        {
            "manifest": manifest,
            "charts_dir": args.charts,
            "out_path": args.out,
            "rows": rows,
            "history": [],
            "lock": threading.Lock(),
        },
    )
    server = ThreadingHTTPServer((args.host, args.port), handler)
    url = f"http://{args.host}:{args.port}/"
    print(f"annotator: {url}")
    print(f"annotations: {args.out}")
    print("press Ctrl+C to stop")
    if args.open:
        webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nstopped")
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
