#!/usr/bin/env python3
"""Tests for the click-to-annotate chart server."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import annotate


def record() -> dict:
    return {
        "file": "chart.png",
        "candles": 3,
        "plot_left": 100,
        "plot_width": 300,
        "price_top": 10,
        "price_bottom": 210,
        "price_low": 0.0,
        "price_high": 300.0,
        "timeframe_secs": 900,
        "bars": [
            [10, 1_600_000_000, 120.0, 90.0],
            [11, 1_600_000_900, 140.0, 100.0],
            [12, 1_600_001_800, 160.0, 110.0],
        ],
    }


def manifest() -> dict:
    return {"charts": [record()]}


class MappingTest(unittest.TestCase):
    def test_click_maps_to_the_candle(self) -> None:
        item = record()
        self.assertEqual(annotate.bar_from_x(item, 100), 10)
        self.assertEqual(annotate.bar_from_x(item, 199), 10)
        self.assertEqual(annotate.bar_from_x(item, 201), 11)
        self.assertEqual(annotate.bar_from_x(item, 399), 12)

    def test_click_outside_the_plot_is_ignored(self) -> None:
        item = record()
        self.assertIsNone(annotate.bar_from_x(item, 99))
        self.assertIsNone(annotate.bar_from_x(item, 401))


class AnnotationStoreTest(unittest.TestCase):
    def test_validate_derives_the_bar_timestamp(self) -> None:
        row = annotate.validate_annotation(
            {"chart": "chart.png", "bar": 11, "action": "entry", "quality": "good", "side": "long", "note": "x"},
            manifest(),
        )
        self.assertEqual(row["marked_time_utc"], "2020-09-13T12:41:40Z")
        self.assertEqual(row["bar"], "11")

    def test_unknown_chart_and_bar_are_rejected(self) -> None:
        with self.assertRaises(ValueError):
            annotate.validate_annotation({"chart": "missing.png", "bar": 10, "action": "entry", "quality": "good", "side": "long"}, manifest())
        with self.assertRaises(ValueError):
            annotate.validate_annotation({"chart": "chart.png", "bar": 99, "action": "entry", "quality": "good", "side": "long"}, manifest())

    def test_upsert_and_delete_are_keyed_by_chart_bar_action_side(self) -> None:
        row = annotate.validate_annotation(
            {"chart": "chart.png", "bar": 10, "action": "entry", "quality": "good", "side": "long"},
            manifest(),
        )
        rows = annotate.upsert([], row)
        self.assertEqual(len(rows), 1)
        replacement = dict(row, quality="bad")
        rows = annotate.upsert(rows, replacement)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["quality"], "bad")
        rows = annotate.delete(rows, {"chart": "chart.png", "bar": "10", "action": "entry", "side": "long"})
        self.assertEqual(rows, [])

    def test_csv_round_trip_keeps_the_annotations_schema(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "annotations.csv"
            row = annotate.validate_annotation(
                {"chart": "chart.png", "bar": 12, "action": "exit", "quality": "good", "side": "long", "note": "high"},
                manifest(),
            )
            annotate.write_annotations(path, [row])
            rows = annotate.read_annotations(path, manifest())
            self.assertEqual(rows[0]["action"], "exit")
            self.assertEqual(rows[0]["note"], "high")
            self.assertEqual(rows[0]["bar"], "12")
            header = path.read_text(encoding="utf-8").splitlines()[0]
            self.assertEqual(header, "chart,bar,marked_time_utc,action,quality,side,note")


class UndoTest(unittest.TestCase):
    def row(self, bar: int = 10, quality: str = "good") -> dict:
        return annotate.validate_annotation(
            {
                "chart": "chart.png",
                "bar": bar,
                "action": "entry",
                "quality": quality,
                "side": "long",
            },
            manifest(),
        )

    def test_undo_new_save_removes_it(self) -> None:
        row = self.row()
        history: list[dict] = []
        rows = annotate.upsert([], row)
        annotate.record_save(history, annotate.annotation_key(row), None)
        self.assertEqual(annotate.undo_last(rows, history), [])
        self.assertEqual(history, [])

    def test_undo_save_restores_the_replaced_row(self) -> None:
        previous = self.row(quality="good")
        replacement = self.row(quality="bad")
        history: list[dict] = []
        rows = annotate.upsert([], previous)
        rows = annotate.upsert(rows, replacement)
        annotate.record_save(history, annotate.annotation_key(replacement), previous)
        rows = annotate.undo_last(rows, history)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["quality"], "good")

    def test_undo_delete_restores_the_row(self) -> None:
        row = self.row()
        history: list[dict] = []
        removed = annotate.find_annotation([row], annotate.annotation_key(row))
        rows = annotate.delete([row], row)
        annotate.record_delete(history, removed)
        rows = annotate.undo_last(rows, history)
        self.assertEqual(rows, [row])

    def test_undo_walks_back_multiple_changes(self) -> None:
        first = self.row(bar=10)
        second = self.row(bar=11)
        history: list[dict] = []
        rows = annotate.upsert([], first)
        annotate.record_save(history, annotate.annotation_key(first), None)
        rows = annotate.upsert(rows, second)
        annotate.record_save(history, annotate.annotation_key(second), None)
        rows = annotate.undo_last(rows, history)
        self.assertEqual([item["bar"] for item in rows], ["10"])
        rows = annotate.undo_last(rows, history)
        self.assertEqual(rows, [])

    def test_empty_history_is_rejected(self) -> None:
        with self.assertRaises(ValueError):
            annotate.undo_last([], [])

    def test_history_is_bounded(self) -> None:
        history: list[dict] = []
        for index in range(annotate.UNDO_LIMIT + 5):
            annotate.push_history(history, {"kind": "delete", "row": {"bar": str(index)}})
        self.assertEqual(len(history), annotate.UNDO_LIMIT)
        self.assertEqual(history[0]["row"]["bar"], "5")


if __name__ == "__main__":
    unittest.main()
