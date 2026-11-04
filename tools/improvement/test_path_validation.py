#!/usr/bin/env python3
"""Unit tests for the model v4 path-validation harness."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import path_validation


def row(
    bar: int,
    close: float,
    high: float,
    low: float,
    atr: float | None = 1.0,
    **extra: float | None,
) -> dict:
    value = {
        "bar": bar,
        "close": close,
        "high": high,
        "low": low,
        "atr": atr,
        "rsi": None,
        "adx_regime": None,
        "adx_slope": None,
        "di_state": None,
        "ema_stack": None,
        "macd_state": None,
        "ema_fast": None,
        "ema_mid": None,
        "ema_slow": None,
    }
    value.update(extra)
    return value


def event(
    bar: int,
    direction: int = 1,
    event_type: int = 3,
    subtype: int = 1,
    source: int = 4,
    reference: float | None = 50.0,
    index: int = 0,
) -> dict:
    return {
        "timeframe": 300,
        "index": index,
        "type": event_type,
        "subtype": subtype,
        "source": source,
        "available": bar,
        "occurred": bar,
        "direction": direction,
        "reference": reference,
        "primary": None,
    }


class PathHitTests(unittest.TestCase):
    def test_long_target_is_a_hit(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5), row(1, 101.0, 102.0, 100.5)]
        self.assertTrue(path_validation.path_hit(rows, 0, 1, 1, 1.0, 1.0))

    def test_long_stop_is_a_miss(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5), row(1, 99.0, 99.5, 98.0)]
        self.assertFalse(path_validation.path_hit(rows, 0, 1, 1, 1.0, 1.0))

    def test_short_target_is_a_hit(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5), row(1, 99.0, 99.5, 98.0)]
        self.assertTrue(path_validation.path_hit(rows, 0, -1, 1, 1.0, 1.0))

    def test_same_bar_target_and_stop_is_a_stop(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5), row(1, 100.0, 102.0, 98.0)]
        self.assertFalse(path_validation.path_hit(rows, 0, 1, 1, 1.0, 1.0))

    def test_timeout_is_a_miss(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5), row(1, 100.0, 100.2, 99.8)]
        self.assertFalse(path_validation.path_hit(rows, 0, 1, 1, 1.0, 1.0))

    def test_missing_atr_is_unavailable(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5, atr=None), row(1, 100.0, 100.5, 99.5)]
        self.assertIsNone(path_validation.path_hit(rows, 0, 1, 1, 1.0, 1.0))

    def test_short_history_is_unavailable(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5)]
        self.assertIsNone(path_validation.path_hit(rows, 0, 1, 1, 1.0, 1.0))

    def test_zero_atr_is_unavailable(self) -> None:
        rows = [row(0, 100.0, 100.5, 99.5, atr=0.0), row(1, 100.0, 100.5, 99.5)]
        self.assertIsNone(path_validation.path_hit(rows, 0, 1, 1, 1.0, 1.0))


class SelectionTests(unittest.TestCase):
    def test_same_bar_same_direction_keeps_one_vote(self) -> None:
        rows = [row(0, 100.0, 101.0, 99.0)]
        events = [event(0, index=0), event(0, index=1)]
        selected, raw = path_validation.select_events(
            events, rows, {"event_type": 3, "subtype": 1, "source": 4, "reference_value": 50.0}
        )
        self.assertEqual(raw, 2)
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0]["index"], 0)

    def test_conflicting_bar_is_dropped(self) -> None:
        rows = [row(0, 100.0, 101.0, 99.0)]
        events = [event(0, direction=1), event(0, direction=-1, index=1)]
        selected, _ = path_validation.select_events(
            events, rows, {"event_type": 3, "subtype": 1, "source": 4, "reference_value": 50.0}
        )
        self.assertEqual(selected, [])

    def test_family_filter_and_reference(self) -> None:
        rows = [row(0, 100.0, 101.0, 99.0)]
        events = [event(0, reference=70.0), event(0, source=13, subtype=2)]
        selected, raw = path_validation.select_events(
            events, rows, {"event_type": 3, "subtype": 1, "source": 4, "reference_value": 50.0}
        )
        self.assertEqual(raw, 0)
        self.assertEqual(selected, [])


class ClusterTests(unittest.TestCase):
    def test_clusters_never_span_bundles(self) -> None:
        entries = [(0, 0, True), (0, 1, False), (1, 0, True), (1, 1, True)]
        stats = path_validation.cluster_stats(entries, 20)
        self.assertEqual(stats["n"], 4)
        self.assertEqual(stats["clusters"], 2)
        self.assertAlmostEqual(stats["hit_rate"], 0.75)

    def test_close_events_share_a_cluster(self) -> None:
        entries = [(0, 0, True), (0, 1, False), (1, 0, True), (1, 25, True)]
        stats = path_validation.cluster_stats(entries, 20)
        self.assertEqual(stats["clusters"], 3)

    def test_empty(self) -> None:
        stats = path_validation.cluster_stats([], 20)
        self.assertEqual(stats["n"], 0)
        self.assertIsNone(stats["hit_rate"])


class StateTests(unittest.TestCase):
    def test_trend_state(self) -> None:
        value = row(
            0,
            100.0,
            101.0,
            99.0,
            adx_regime=1.0,
            adx_slope=1.0,
            di_state=1.0,
            ema_stack=1.0,
            macd_state=1.0,
        )
        self.assertEqual(
            path_validation.state_key("trend", value, 1, 16), ("trend", 1, None)
        )
        self.assertIsNone(path_validation.state_key("trend", value, -1, 16))

    def test_macd_state(self) -> None:
        value = row(0, 100.0, 101.0, 99.0, macd_state=-1.0)
        self.assertEqual(
            path_validation.state_key("macd_state", value, -1, 13),
            ("macd_state", -1, None),
        )

    def test_rsi_side(self) -> None:
        value = row(0, 100.0, 101.0, 99.0, rsi=55.0)
        self.assertEqual(
            path_validation.state_key("rsi_side", value, 1, 4), ("rsi_side", 1, None)
        )
        self.assertIsNone(path_validation.state_key("rsi_side", value, -1, 4))

    def test_ema_side(self) -> None:
        value = row(0, 101.0, 101.5, 99.5, ema_fast=100.0)
        self.assertEqual(
            path_validation.state_key("ema_side", value, 1, 1), ("ema_side", 1, 1)
        )
        value = row(0, 99.0, 101.5, 98.5, ema_fast=100.0)
        self.assertIsNone(path_validation.state_key("ema_side", value, 1, 1))


class ControlTests(unittest.TestCase):
    def test_random_draws_are_deterministic(self) -> None:
        rows = [
            row(index, 100.0 + index, 100.5 + index, 99.5 + index, atr=1.0)
            for index in range(40)
        ]
        hits = path_validation.barrier_arrays(rows, 2, 1.0, 1.0)
        bundle = path_validation.Bundle(
            instrument="TEST",
            fold=2020,
            partition="model_dev",
            config="adx25",
        )
        bundle.hits[300] = hits
        bundle.bases[300] = {direction: path_validation.counts(hits[direction]) for direction in (-1, 1)}
        bundle.valid[300] = {
            direction: [index for index, hit in enumerate(hits[direction]) if hit is not None]
            for direction in (-1, 1)
        }
        selected = [(0, path_validation.Selected(5, 1, 4, 0, True, ("rsi_side", 1, None)))]
        options = path_validation.Options(random_draws=20, shuffle_draws=20, seed=0)
        first = path_validation.random_draws([bundle], 300, selected, options)
        second = path_validation.random_draws([bundle], 300, selected, options)
        self.assertEqual(first, second)
        self.assertEqual(
            path_validation.shuffle_draws([bundle], 300, selected, options),
            path_validation.shuffle_draws([bundle], 300, selected, options),
        )


class EndToEndTests(unittest.TestCase):
    def test_load_and_evaluate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            rows = []
            for index in range(60):
                close = 100.0 + index * 0.5
                rows.append(
                    row(
                        index,
                        close,
                        close + 0.4,
                        close - 0.4,
                        atr=1.0,
                        rsi=60.0,
                        adx_regime=1.0,
                        adx_slope=1.0,
                        di_state=1.0,
                        ema_stack=1.0,
                        macd_state=1.0,
                        ema_fast=close - 0.5,
                        ema_mid=close - 1.0,
                        ema_slow=close - 1.5,
                    )
                )
            with (run / "readings.csv").open("w", newline="") as handle:
                writer = csv.DictWriter(
                    handle,
                    fieldnames=[
                        "timeframe_secs",
                        "bar",
                        "open",
                        "high",
                        "low",
                        "close",
                        "volume",
                        "ema_fast",
                        "ema_mid",
                        "ema_slow",
                        "rsi",
                        "atr",
                        "rvol",
                        "macd_line",
                        "macd_signal",
                        "macd_histogram",
                        "adx",
                        "di_plus",
                        "di_minus",
                        "ema_stack",
                        "rsi_zone",
                        "macd_state",
                        "adx_regime",
                        "adx_slope",
                        "di_state",
                        "progression_stage_code",
                        "progression_leg_direction",
                    ],
                )
                writer.writeheader()
                for value in rows:
                    writer.writerow(
                        {
                            "timeframe_secs": 300,
                            "bar": value["bar"],
                            "open": value["close"],
                            "high": value["high"],
                            "low": value["low"],
                            "close": value["close"],
                            "volume": 1.0,
                            "ema_fast": value["ema_fast"],
                            "ema_mid": value["ema_mid"],
                            "ema_slow": value["ema_slow"],
                            "rsi": value["rsi"],
                            "atr": value["atr"],
                            "rvol": 1.0,
                            "macd_line": 1.0,
                            "macd_signal": 0.5,
                            "macd_histogram": 0.5,
                            "adx": 30.0,
                            "di_plus": 25.0,
                            "di_minus": 10.0,
                            "ema_stack": value["ema_stack"],
                            "rsi_zone": 2.0,
                            "macd_state": value["macd_state"],
                            "adx_regime": value["adx_regime"],
                            "adx_slope": value["adx_slope"],
                            "di_state": value["di_state"],
                            "progression_stage_code": "",
                            "progression_leg_direction": "",
                        }
                    )
            with (run / "events.csv").open("w", newline="") as handle:
                writer = csv.DictWriter(
                    handle,
                    fieldnames=[
                        "timeframe_secs",
                        "event_index",
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
                )
                writer.writeheader()
                writer.writerow(
                    {
                        "timeframe_secs": 300,
                        "event_index": 0,
                        "event_type_code": 3,
                        "event_subtype_code": 1,
                        "source_code": 4,
                        "occurred_bar_index": 10,
                        "available_bar_index": 10,
                        "direction_code": 1,
                        "reference_code": "",
                        "primary_value": 55.0,
                        "reference_value": 50.0,
                        "data_completeness": 1.0,
                    }
                )
            cell = {
                "instrument": "TEST",
                "fold": 2020,
                "partition": "model_dev",
                "config": "adx25",
                "run": str(run),
            }
            families = [
                {
                    "id": "h3_rsi50",
                    "event_type": 3,
                    "subtype": 1,
                    "source": 4,
                    "reference_value": 50.0,
                    "state": "rsi_side",
                }
            ]
            options = path_validation.Options(random_draws=10, shuffle_draws=10)
            bundle = path_validation.load_bundle(cell, families, options, {300})
            self.assertEqual(bundle.selected[("h3_rsi50", 300)][0].hit, True)
            results = path_validation.evaluate(
                [bundle], families, {300}, options, path_validation.primary_z()
            )
            self.assertEqual(results[0]["n"], 1)
            self.assertEqual(results[0]["hits"], 1)
            self.assertEqual(results[0]["state_missing"], 0)


class MatrixTests(unittest.TestCase):
    def test_expand_cells(self) -> None:
        matrix = {
            "cells": [
                {
                    "instrument": "BTCUSD",
                    "fold": 2018,
                    "partition": "model_dev",
                    "runs": {"adx25": "a/model", "adx20": "b/model"},
                }
            ]
        }
        cells = path_validation.expand_cells(matrix, None)
        self.assertEqual(len(cells), 2)
        self.assertEqual({cell["config"] for cell in cells}, {"adx25", "adx20"})
        only = path_validation.expand_cells(matrix, "adx25")
        self.assertEqual(len(only), 1)
        self.assertEqual(only[0]["run"], "a/model")


if __name__ == "__main__":
    unittest.main()
