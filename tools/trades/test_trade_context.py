#!/usr/bin/env python3
"""Tests for the trade-context, annotation, and exit-quality tools."""

from __future__ import annotations

import csv
import io
import json
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime, timezone
from pathlib import Path

import annotations
import exit_quality
import trade_context
from bundle import load_bundle

BASE = 1_600_000_000
HEADER = [
    "timeframe_secs",
    "bar",
    "timestamp",
    "open",
    "high",
    "low",
    "close",
    "volume",
    "ema_fast",
    "ema_mid",
    "ema_slow",
    "atr",
    "rsi",
    "rsi_zone",
    "macd_state",
    "macd_histogram",
    "adx",
    "adx_regime",
    "rvol_ratio",
    "rvol_regime",
    "atr_ratio",
    "vwap_distance_atr",
    "range_position",
    "drawdown_atr",
    "path_efficiency",
    "level_touches",
    "structure_state",
    "vol_regime",
    "ema_stack",
]
TRADES_HEADER = [
    "trade_id",
    "side",
    "quantity",
    "entry_bar",
    "entry_time",
    "entry_price",
    "exit_bar",
    "exit_time",
    "exit_price",
    "gross_pnl",
    "execution_fees",
    "commission",
    "funding",
    "net_pnl",
    "return_fraction",
    "hold_bars",
    "exit_reason",
]


def write_csv(path: Path, header: list[str], rows: list[list[object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        writer.writerows(rows)


def build_bundle(root: Path) -> Path:
    base = 1_600_000_000
    readings = []
    for bar in range(400):
        price = 100.0 + bar
        readings.append(
            [
                900,
                bar,
                base + bar * 900,
                price,
                price + 2,
                price - 2,
                price + 1,
                10,
                price - 5,
                price - 10,
                price - 20,
                1.0,
                50,
                0,
                1,
                1,
                25,
                1,
                1.5,
                0,
                1,
                0.5,
                0.5,
                -0.1,
                0.5,
                1,
                1,
                1,
                1,
            ]
        )
    write_csv(root / "model" / "readings.csv", HEADER, readings)
    write_csv(
        root / "model" / "events.csv",
        [
            "timeframe_secs",
            "event_index",
            "event_code",
            "event_type_code",
            "event_subtype_code",
            "source_code",
            "occurred_bar_index",
            "available_bar_index",
            "direction_code",
            "reference_code",
            "primary_value",
            "reference_value",
            "data_completeness",
        ],
        [[900, 0, 17, 3, 7, 5, 99, 99, 1, "", 3.0, 0.0, 1.0]],
    )
    trades = [
        [1, "long", 1, 100, base + 100 * 900, 200.0, 110, base + 110 * 900, 210.0, 0, 0, 0, 0, 10.0, 0.05, 10, "target"],
        [2, "short", 1, 200, base + 200 * 900, 300.0, 210, base + 210 * 900, 290.0, 0, 0, 0, 0, 10.0, 0.0333333333, 10, "target"],
    ]
    write_csv(root / "portfolio" / "trades.csv", TRADES_HEADER, trades)
    write_csv(
        root / "execution" / "orders.csv",
        ["order_id", "submitted_bar", "submitted_time", "side", "quantity", "reason", "trigger_kind", "trigger_price", "state", "filled_bar", "filled_time", "fill_id", "rejection_reason"],
        [
            [0, 99, base + 99 * 900, "buy", 1, "entry", "market", "", "filled", 100, base + 100 * 900, 0, ""],
            [1, 100, base + 100 * 900, "sell", 1, "take_profit", "target", 215, "filled", 110, base + 110 * 900, 1, ""],
            [2, 199, base + 199 * 900, "sell", 1, "entry", "market", "", "filled", 200, base + 200 * 900, 2, ""],
            [3, 200, base + 200 * 900, "buy", 1, "take_profit", "target", 285, "filled", 210, base + 210 * 900, 3, ""],
        ],
    )
    write_csv(
        root / "execution" / "fills.csv",
        ["fill_id", "side", "quantity", "reference_price", "price", "slippage", "fee", "bar", "time", "reason"],
        [
            [0, "buy", 1, 200, 200, 0, 0, 100, base + 100 * 900, "entry"],
            [1, "sell", 1, 210, 210, 0, 0, 110, base + 110 * 900, "take_profit"],
            [2, "sell", 1, 300, 300, 0, 0, 200, base + 200 * 900, "entry"],
            [3, "buy", 1, 290, 290, 0, 0, 210, base + 210 * 900, "take_profit"],
        ],
    )
    decisions = [
        {"schema": "quantrule.strategy-decision/v1", "bar_index": bar, "decision": "hold"}
        for bar in range(400)
    ]
    decisions[99] = {"schema": "quantrule.strategy-decision/v1", "bar_index": 99, "decision": "enter", "reason": "entry"}
    decisions[109] = {"schema": "quantrule.strategy-decision/v1", "bar_index": 109, "decision": "hold"}
    decisions[199] = {"schema": "quantrule.strategy-decision/v1", "bar_index": 199, "decision": "enter", "reason": "entry"}
    decisions[209] = {"schema": "quantrule.strategy-decision/v1", "bar_index": 209, "decision": "hold"}
    (root / "strategy").mkdir(parents=True)
    (root / "strategy" / "decisions.ndjson").write_text(
        "".join(json.dumps(record) + "\n" for record in decisions), encoding="utf-8"
    )
    (root / "manifest.json").write_text(
        json.dumps({"data_version": "test-v1", "config_id": "test-config", "run": "test"}) + "\n",
        encoding="utf-8",
    )
    return root


class ContextTest(unittest.TestCase):
    def test_context_ends_at_the_decision_bar_and_post_starts_after_fill(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            bundle = load_bundle(build_bundle(Path(directory) / "bundle"))
            self.assertEqual(len(bundle.readings), 400)
            self.assertEqual(bundle.trade(1)["side"], "long")
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                code = trade_context.main(
                    [
                        "trade_context.py",
                        "--bundle",
                        str(Path(directory) / "bundle"),
                        "--out",
                        str(Path(directory) / "out"),
                        "--window",
                        "250",
                        "--post",
                        "250",
                        "--trade-id",
                        "1",
                        "--event-registry",
                        str(Path(directory) / "missing.json"),
                    ]
                )
            self.assertEqual(code, 0)
            root = Path(directory) / "out" / "trade-1"
            with (root / "entry" / "context.csv").open() as handle:
                rows = list(csv.DictReader(handle))
            self.assertEqual(len(rows), 100)
            self.assertEqual(int(rows[-1]["bar"]), 99)
            self.assertLessEqual(int(rows[-1]["bar"]), 99)
            with (root / "entry" / "post.csv").open() as handle:
                post = list(csv.DictReader(handle))
            self.assertEqual(len(post), 250)
            self.assertEqual(int(post[0]["bar"]), 100)
            summary = json.loads((root / "entry" / "summary.json").read_text())
            self.assertEqual(summary["decision_bar"], 99)
            self.assertEqual(summary["readings"]["ema_stack"], 1)
            self.assertEqual(summary["events_at_decision"][0]["code"], 17)

    def test_an_annotation_maps_to_the_nearest_bar(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_bundle(root / "bundle")
            annotation = root / "annotations.csv"
            marked = datetime.fromtimestamp(BASE + 99 * 900, tz=timezone.utc).isoformat()
            annotation.write_text(
                "chart,marked_time_utc,action,quality,side,note\n"
                f"chart.png,{marked},entry,good,long,marked low\n",
                encoding="utf-8",
            )
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                code = annotations.main(
                    [
                        "annotations.py",
                        "--bundle",
                        str(root / "bundle"),
                        "--annotations",
                        str(annotation),
                        "--out",
                        str(root / "out"),
                        "--event-registry",
                        str(root / "missing.json"),
                    ]
                )
            self.assertEqual(code, 0)
            summary = json.loads((root / "out" / "annotation-001" / "summary.json").read_text())
            self.assertEqual(summary["annotation"]["quality"], "good")
            self.assertGreaterEqual(summary["context_bars"], 1)

    def test_exit_quality_has_one_row_per_trade(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_bundle(root / "bundle")
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                code = exit_quality.main(
                    [
                        "exit_quality.py",
                        "--bundle",
                        str(root / "bundle"),
                        "--out",
                        str(root / "exit_quality.csv"),
                    ]
                )
            self.assertEqual(code, 0)
            with (root / "exit_quality.csv").open() as handle:
                rows = list(csv.DictReader(handle))
            self.assertEqual(len(rows), 2)
            self.assertEqual(rows[0]["exit_reason"], "target")
            self.assertIn("post_50_mfe", rows[0])
            self.assertIn("exit_efficiency", rows[0])


if __name__ == "__main__":
    unittest.main()
