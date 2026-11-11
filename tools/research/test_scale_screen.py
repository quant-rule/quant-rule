#!/usr/bin/env python3
"""Unit tests for the SBE-01 scale screen."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import scale_screen


def write_bundle(
    root: Path,
    year: int,
    events: list[dict[str, object]],
    readings: list[dict[str, object]],
    *,
    nested: bool = False,
) -> Path:
    bundle = root / str(year)
    if nested:
        bundle = bundle / "model"
    bundle.mkdir(parents=True)
    event_fields = [
        "timeframe_secs",
        "event_type_code",
        "event_subtype_code",
        "source_code",
        "direction_code",
        "available_bar_index",
        "primary_value",
        "reference_value",
    ]
    with (bundle / "events.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=event_fields)
        writer.writeheader()
        writer.writerows(events)
    reading_fields = ["timeframe_secs", "bar", "atr"]
    with (bundle / "readings.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=reading_fields)
        writer.writeheader()
        writer.writerows(readings)
    return bundle


def event(
    bar: int,
    high: float,
    low: float,
    *,
    subtype: int = 9,
    source: int = 5,
    direction: int = 0,
    timeframe: int = 300,
) -> dict[str, object]:
    return {
        "timeframe_secs": timeframe,
        "event_type_code": 3,
        "event_subtype_code": subtype,
        "source_code": source,
        "direction_code": direction,
        "available_bar_index": bar,
        "primary_value": high,
        "reference_value": low,
    }


def reading(bar: int, atr: float, *, timeframe: int = 300) -> dict[str, object]:
    return {"timeframe_secs": timeframe, "bar": bar, "atr": atr}


def onset(bar: int = 10, high: float = 101_000.0, low: float = 100_000.0) -> scale_screen.Onset:
    return scale_screen.Onset(year=2018, bar=bar, high=high, low=low, atr=100.0)


class MetricTests(unittest.TestCase):
    def test_the_example_geometry_passes_at_six_bps(self) -> None:
        samples = scale_screen.samples_for(
            [onset()], 0.3, 0.6, 6.0
        )
        self.assertEqual(len(samples), 2)
        long = next(sample for sample in samples if sample.side == "long")
        short = next(sample for sample in samples if sample.side == "short")
        self.assertAlmostEqual(long.width, 1_000.0)
        self.assertAlmostEqual(long.width_atr, 10.0)
        self.assertAlmostEqual(long.stop_bps, 0.3 * 1_000.0 / 101_000.0 * 10_000.0)
        self.assertAlmostEqual(long.target_bps, 0.6 * 1_000.0 / 101_000.0 * 10_000.0)
        self.assertAlmostEqual(long.cost_r, 0.202)
        self.assertAlmostEqual(long.net_target, 600.0 - 60.6)
        self.assertAlmostEqual(long.breakeven, (300.0 + 60.6) / (300.0 + 60.6 + 600.0 - 60.6))
        self.assertAlmostEqual(short.cost_r, 0.2)
        self.assertAlmostEqual(short.breakeven, 0.4)
        self.assertLess(long.breakeven, 0.5)
        self.assertLess(short.breakeven, 0.5)

    def test_a_target_inside_the_cost_is_infeasible(self) -> None:
        samples = scale_screen.samples_for(
            [onset(high=100_010.0, low=100_000.0)], 0.3, 0.6, 6.0
        )
        for sample in samples:
            self.assertLessEqual(sample.net_target, 0.0)
            self.assertIsNone(sample.breakeven)

    def test_a_short_mirrors_the_long_distances(self) -> None:
        samples = scale_screen.samples_for([onset()], 0.3, 0.6, 6.0)
        long = next(sample for sample in samples if sample.side == "long")
        short = next(sample for sample in samples if sample.side == "short")
        self.assertAlmostEqual(long.stop_price, 101_000.0 - 300.0)
        self.assertAlmostEqual(long.target_price, 101_000.0 + 600.0)
        self.assertAlmostEqual(short.stop_price, 100_000.0 + 300.0)
        self.assertAlmostEqual(short.target_price, 100_000.0 - 600.0)
        self.assertAlmostEqual(long.width, short.width)

    def test_the_atr_control_uses_the_same_reward_risk(self) -> None:
        samples = scale_screen.samples_for([onset()], 0.3, 0.6, 6.0)
        long = next(sample for sample in samples if sample.side == "long")
        self.assertAlmostEqual(long.atr_stop_bps, 100.0 / 101_000.0 * 10_000.0)
        self.assertAlmostEqual(long.atr_target_bps, 200.0 / 101_000.0 * 10_000.0)
        self.assertAlmostEqual(long.atr_cost_r, 0.606)
        self.assertGreater(long.atr_breakeven, 0.5)


class ParsingTests(unittest.TestCase):
    def test_events_are_filtered_to_the_registered_onset_identity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = write_bundle(
                root,
                2018,
                [
                    event(10, 101_000.0, 100_000.0),
                    event(11, 102_000.0, 101_000.0, subtype=8),
                    event(12, 103_000.0, 102_000.0, source=6),
                    event(13, 104_000.0, 103_000.0, direction=1),
                    event(14, 105_000.0, 104_000.0, timeframe=900),
                ],
                [reading(10, 100.0)],
            )
            events = scale_screen.parse_events(bundle / "events.csv", 300, 2018)
            self.assertEqual([event.bar for event in events], [10])

    def test_zero_range_and_missing_atr_are_skipped(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = write_bundle(
                root,
                2018,
                [
                    event(10, 100_000.0, 100_000.0),
                    event(11, 101_000.0, 100_000.0),
                    event(12, 102_000.0, 101_000.0),
                ],
                [reading(11, 100.0), reading(12, 0.0)],
            )
            events = scale_screen.load_year(bundle, 2018, 300)
            self.assertEqual([event.bar for event in events], [11])

    def test_a_model_report_subdirectory_is_read(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(
                root,
                2018,
                [event(10, 101_000.0, 100_000.0)],
                [reading(10, 100.0)],
                nested=True,
            )
            events = scale_screen.load_year(root / "2018", 2018, 300)
            self.assertEqual(len(events), 1)
            self.assertEqual(events[0].bar, 10)

    def test_percentile_interpolates(self) -> None:
        values = [1.0, 2.0, 3.0, 4.0]
        self.assertAlmostEqual(scale_screen.percentile(values, 0.0), 1.0)
        self.assertAlmostEqual(scale_screen.percentile(values, 0.5), 2.5)
        self.assertAlmostEqual(scale_screen.percentile(values, 0.75), 3.25)
        self.assertAlmostEqual(scale_screen.percentile(values, 1.0), 4.0)
        self.assertIsNone(scale_screen.percentile([], 0.5))


class VerdictTests(unittest.TestCase):
    def _samples(self, count: int, width: float = 1_000.0) -> list[scale_screen.Sample]:
        events = [
            scale_screen.Onset(
                year=2018 + index % 5,
                bar=index,
                high=101_000.0,
                low=101_000.0 - width,
                atr=100.0,
            )
            for index in range(count)
        ]
        return scale_screen.samples_for(events, 0.3, 0.6, 6.0)

    def test_the_pre_registered_geometry_is_feasible_on_a_synthetic_sample(self) -> None:
        aggregate = scale_screen.aggregate(self._samples(250))
        verdict, reasons = scale_screen.verdict(aggregate)
        self.assertEqual(verdict, "feasible", reasons)

    def test_a_small_sample_is_underpowered(self) -> None:
        aggregate = scale_screen.aggregate(self._samples(20))
        verdict, reasons = scale_screen.verdict(aggregate)
        self.assertEqual(verdict, "underpowered")
        self.assertTrue(any("valid events" in reason for reason in reasons))

    def test_a_cost_negative_target_is_infeasible(self) -> None:
        aggregate = scale_screen.aggregate(self._samples(250, width=10.0))
        verdict, reasons = scale_screen.verdict(aggregate)
        self.assertEqual(verdict, "infeasible")
        self.assertTrue(any("net target" in reason for reason in reasons))

    def test_a_single_small_year_is_underpowered(self) -> None:
        samples = self._samples(200)
        samples = [sample for sample in samples if sample.year != 2022]
        samples += [
            scale_screen.Sample(
                **{
                    **sample.__dict__,
                    "year": 2022,
                }
            )
            for sample in samples[:4]
        ]
        aggregate = scale_screen.aggregate(samples)
        verdict, reasons = scale_screen.verdict(aggregate)
        self.assertEqual(verdict, "underpowered")
        self.assertTrue(any("2022" in reason for reason in reasons))


class ReportTests(unittest.TestCase):
    def test_the_same_input_produces_the_same_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(
                root,
                2018,
                [event(10, 101_000.0, 100_000.0)],
                [reading(10, 100.0)],
            )
            first_aggregate, _ = scale_screen.evaluate_bundles(
                root, 0.3, 0.6, 6.0
            )
            second_aggregate, _ = scale_screen.evaluate_bundles(
                root, 0.3, 0.6, 6.0
            )
            first = json.dumps(first_aggregate, sort_keys=True)
            second = json.dumps(second_aggregate, sort_keys=True)
            self.assertEqual(first, second)

    def test_the_report_is_written(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundles = root / "bundles"
            write_bundle(
                bundles,
                2018,
                [event(10, 101_000.0, 100_000.0)],
                [reading(10, 100.0)],
            )
            out = root / "out"
            document = scale_screen.run(
                bundles=bundles,
                stop_fraction=0.3,
                target_fraction=0.6,
                cost_bps=6.0,
                sensitivity_bps=9.0,
                out=out,
            )
            self.assertTrue((out / "scale_screen.json").is_file())
            self.assertTrue((out / "scale_screen.md").is_file())
            self.assertEqual(document["schema"], scale_screen.SCHEMA)
            base = document["base"]
            self.assertIsInstance(base, dict)
            self.assertEqual(base["verdict"], "underpowered")

    def test_an_empty_bundle_root_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(scale_screen.ScaleScreenError):
                scale_screen.evaluate_bundles(root, 0.3, 0.6, 6.0)


if __name__ == "__main__":
    unittest.main()
