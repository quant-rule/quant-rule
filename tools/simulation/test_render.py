#!/usr/bin/env python3
"""Tests for the deterministic simulation chart renderer."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from bundle import load_bundle
from render import Image, contact_sheet, render_window, render_windows
from window_select import Window, WindowSpec, select_windows

FIXTURE = Path(__file__).parent / "fixtures" / "mini"


def one_window(bundle) -> Window:
    return Window(
        window_id="w001",
        start_index=0,
        end_index=200,
        candidate_index=60,
        events=(),
    )


@unittest.skipIf(Image is None, "Pillow is not installed")
class RenderTest(unittest.TestCase):
    def setUp(self) -> None:
        self.bundle = load_bundle(FIXTURE)

    def test_render_geometry_and_manifest(self) -> None:
        marks = [
            {
                "chart": "sim-w001.png",
                "bar": "60",
                "marked_time_utc": "",
                "action": "entry",
                "quality": "good",
                "side": "long",
                "note": "test",
            }
        ]
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            record = render_window(
                self.bundle,
                one_window(self.bundle),
                "sim-w001.png",
                out,
                "SIM-RENDER",
                marks,
                {},
                ("ema", "rsi"),
            )
            self.assertTrue((out / "sim-w001.png").is_file())
        self.assertEqual(record["file"], "sim-w001.png")
        self.assertEqual(record["candles"], 200)
        self.assertEqual(record["plot_left"], 110)
        self.assertGreater(record["plot_width"], 0)
        self.assertGreater(record["price_high"], record["price_low"])
        self.assertEqual(len(record["bars"]), 200)
        self.assertEqual(len(record["sha256"]), 64)

    def test_render_is_byte_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            one = render_window(
                self.bundle,
                one_window(self.bundle),
                "sim-w001.png",
                Path(first),
                "SIM-RENDER",
                [],
                {},
                ("ema",),
            )
            two = render_window(
                self.bundle,
                one_window(self.bundle),
                "sim-w001.png",
                Path(second),
                "SIM-RENDER",
                [],
                {},
                ("ema",),
            )
        self.assertEqual(one["sha256"], two["sha256"])

    def test_render_windows_writes_manifest_and_sheet(self) -> None:
        selection = select_windows(
            self.bundle,
            WindowSpec(
                event_codes=(149,),
                window_bars=200,
                candidate_zone=(60, 100),
                max_windows=3,
                min_windows=1,
                overlap_bars=120,
            ),
            {149: "level.fib_test"},
        )
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            charts, manifest = render_windows(
                self.bundle,
                selection,
                [],
                out,
                "SIM-RENDER",
                {149: "level.fib_test"},
                ("ema",),
            )
            self.assertTrue((out / "manifest.json").is_file())
            self.assertTrue((out / "index.csv").is_file())
            contact_sheet(out, manifest, out / "contact-sheet.png")
            self.assertTrue((out / "contact-sheet.png").is_file())
        self.assertEqual(len(charts), 3)
        self.assertEqual(manifest["schema"], "quantrule.simulation-charts/v1")
        self.assertTrue(charts[0]["zoom_file"].endswith("-zoom.png"))


if __name__ == "__main__":
    unittest.main()
