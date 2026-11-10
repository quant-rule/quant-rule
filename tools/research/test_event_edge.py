#!/usr/bin/env python3
"""Unit tests for the configurable event-edge screen."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import event_edge
import raw_edge


def bar(
    index: int,
    high: float = 101.0,
    low: float = 99.0,
    close: float = 100.0,
    atr: float = 1.0,
    ema_stack: int = 1,
    structure_state: int = 1,
) -> raw_edge.Bar:
    return raw_edge.Bar(
        index=index,
        open=100.0,
        high=high,
        low=low,
        close=close,
        atr=atr,
        ema_stack=ema_stack,
        structure_state=structure_state,
    )


def condition(event_type: int, subtype: int, source: int, direction: int) -> event_edge.EventCondition:
    return event_edge.EventCondition(event_type, subtype, source, direction)


class SpecTests(unittest.TestCase):
    def test_a_spec_parses(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "spec.json"
            path.write_text(
                json.dumps(
                    {
                        "schema": "quantrule.event-edge-signal/v1",
                        "id": "fixture",
                        "side": "long",
                        "window": 2,
                        "events": [
                            {"type": 1, "subtype": 3, "source": 8, "direction": -1},
                            {"type": 1, "subtype": 2, "source": 8, "direction": 1},
                        ],
                    }
                )
            )
            spec = event_edge.load_spec(path)
            self.assertEqual(spec.id, "fixture")
            self.assertEqual(spec.side, "long")
            self.assertEqual(len(spec.events), 2)

    def test_an_unsupported_subtype_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "spec.json"
            path.write_text(
                json.dumps(
                    {
                        "schema": "quantrule.event-edge-signal/v1",
                        "id": "fixture",
                        "side": "long",
                        "window": 1,
                        "events": [
                            {"type": 3, "subtype": 23, "source": 51, "direction": 1}
                        ],
                    }
                )
            )
            with self.assertRaises(ValueError):
                event_edge.load_spec(path)


class SignalTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = event_edge.SignalSpec(
            id="h1-long",
            side="long",
            window=2,
            events=(
                condition(1, 3, 8, -1),
                condition(1, 2, 8, 1),
            ),
        )

    def test_break_then_cross_matches(self) -> None:
        bundle = event_edge.EventBundle(
            label="fixture",
            bars=(bar(0), bar(1), bar(2)),
            events={
                (1, 3, 8): {0: frozenset({-1})},
                (1, 2, 8): {1: frozenset({1})},
            },
        )
        self.assertEqual(event_edge.signal_bars(bundle, self.spec), [1])

    def test_cross_then_break_also_matches_without_order(self) -> None:
        bundle = event_edge.EventBundle(
            label="fixture",
            bars=(bar(0), bar(1), bar(2)),
            events={
                (1, 3, 8): {1: frozenset({-1})},
                (1, 2, 8): {0: frozenset({1})},
            },
        )
        self.assertEqual(event_edge.signal_bars(bundle, self.spec), [1])

    def test_a_wrong_direction_does_not_match(self) -> None:
        bundle = event_edge.EventBundle(
            label="fixture",
            bars=(bar(0), bar(1), bar(2)),
            events={
                (1, 3, 8): {0: frozenset({1})},
                (1, 2, 8): {1: frozenset({1})},
            },
        )
        self.assertEqual(event_edge.signal_bars(bundle, self.spec), [])

    def test_a_state_filter_is_applied(self) -> None:
        spec = event_edge.SignalSpec(
            id="h1-long",
            side="long",
            window=1,
            events=(condition(1, 1, 57, 1),),
            states=(("ema_stack", 1),),
        )
        bundle = event_edge.EventBundle(
            label="fixture",
            bars=(bar(0, ema_stack=1), bar(1, ema_stack=-1)),
            events={(1, 1, 57): {0: frozenset({1}), 1: frozenset({1})}},
        )
        self.assertEqual(event_edge.signal_bars(bundle, spec), [0])


class ParsingTests(unittest.TestCase):
    def test_events_are_keyed_by_identity_and_available_bar(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.csv"
            with path.open("w", newline="", encoding="utf-8") as handle:
                writer = csv.writer(handle)
                writer.writerow(
                    [
                        "timeframe_secs",
                        "event_type_code",
                        "event_subtype_code",
                        "source_code",
                        "available_bar_index",
                        "direction_code",
                    ]
                )
                writer.writerow([900, 1, 3, 8, 7, -1])
                writer.writerow([900, 1, 2, 8, 8, 1])
                writer.writerow([60, 1, 1, 57, 99, 1])
            events = event_edge.parse_all_events(path)
            self.assertEqual(events[(1, 3, 8)][7], frozenset({-1}))
            self.assertEqual(events[(1, 2, 8)][8], frozenset({1}))
            self.assertNotIn((1, 1, 57), events)


class EvaluationTests(unittest.TestCase):
    def test_evaluation_reports_sample_and_controls(self) -> None:
        bundle = event_edge.EventBundle(
            label="fixture",
            bars=(
                bar(0),
                bar(1),
                bar(2, high=104.0, low=100.0, close=103.0),
                bar(3, high=105.0, low=100.0, close=104.0),
            ),
            events={(1, 1, 57): {0: frozenset({1})}},
        )
        spec = event_edge.SignalSpec(
            id="fixture",
            side="long",
            window=1,
            events=(condition(1, 1, 57, 1),),
        )
        result = event_edge.evaluate([bundle], spec, (1, 2), 2.0, 1.0, 2, 6.0)
        self.assertEqual(result["signal"]["id"], "fixture")
        self.assertGreater(result["sample"]["signals"], 0)
        self.assertIn("all_bars", result)
        self.assertIn("non_overlapping", result)


if __name__ == "__main__":
    unittest.main()
