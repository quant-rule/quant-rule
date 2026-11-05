"""Tests for the model spec preflight."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import spec_preflight
from spec_preflight import PreflightError, preflight

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "crates/quantrule-shell/tests/fixtures/config-run"
CONFIG = FIXTURE / "run.json"
SPEC = FIXTURE / "spec.json"
STATE = REPO_ROOT / "crates/quantrule-shell/tests/fixtures/state_m1.csv"


def load_spec() -> dict:
    return json.loads(SPEC.read_text(encoding="utf-8"))


def write_json(directory: Path, name: str, document: dict) -> Path:
    path = directory / name
    path.write_text(json.dumps(document), encoding="utf-8")
    return path


def write_data(directory: Path, source: Path) -> Path:
    data = json.loads((FIXTURE / "data.json").read_text(encoding="utf-8"))
    data["source"]["path"] = str(source)
    return write_json(directory, "data.json", data)


def write_run(directory: Path) -> Path:
    return write_json(directory, "run.json", {"data": "data.json"})


class SpecPreflightTest(unittest.TestCase):
    def test_the_fixture_passes(self) -> None:
        result = preflight(CONFIG, SPEC, REPO_ROOT)
        self.assertEqual(result.spec_id, "fixture-spec")
        self.assertEqual(result.ladder, (300, 900))
        self.assertEqual(result.evaluation_timeframe, 300)
        self.assertEqual(result.condition_count, 2)
        self.assertEqual(result.event_count, 1)
        self.assertEqual(result.bars, 260)
        self.assertEqual(result.data_timeframes, (60, 300, 900, 3600))
        registry = json.loads(
            (REPO_ROOT / "docs/registry/indicators.json").read_text(encoding="utf-8")
        )
        self.assertEqual(result.registry_version, registry["version"])

    def test_an_unknown_event_fails(self) -> None:
        spec = load_spec()
        spec["conditions"][0]["when"]["event"]["id"] = "signal.missing"
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            path = write_json(directory, "spec.json", spec)
            with self.assertRaisesRegex(PreflightError, "not in the registry"):
                preflight(CONFIG, path, REPO_ROOT)

    def test_a_ladder_missing_from_the_data_fails(self) -> None:
        spec = load_spec()
        spec["ladder"] = [120, 300]
        spec["conditions"][0]["when"]["event"]["timeframe"] = 300
        spec["conditions"][1]["when"]["state"]["timeframe"] = 120
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            path = write_json(directory, "spec.json", spec)
            with self.assertRaisesRegex(PreflightError, "not in the data configuration"):
                preflight(CONFIG, path, REPO_ROOT)

    def test_a_diagnostic_event_is_allowed(self) -> None:
        spec = load_spec()
        spec["conditions"] = [
            {
                "id": "diagnostic",
                "when": {
                    "event": {"id": "signal.path_efficiency", "timeframe": 300}
                },
            }
        ]
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            path = write_json(directory, "spec.json", spec)
            result = preflight(CONFIG, path, REPO_ROOT)
            self.assertEqual(result.event_count, 1)

    def test_a_missing_source_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            data = write_data(directory, directory / "missing.csv")
            run = write_run(directory)
            with self.assertRaisesRegex(PreflightError, "cannot read the source"):
                preflight(run, SPEC, REPO_ROOT)

    def test_a_non_ascending_source_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            source = directory / "source.csv"
            source.write_text(
                "Date,Open,High,Low,Close,Volume\n"
                "2025-01-01 00:01:00+00:00,1,1,1,1,1\n"
                "2025-01-01 00:00:00+00:00,1,1,1,1,1\n",
                encoding="utf-8",
            )
            write_data(directory, source)
            run = write_run(directory)
            with self.assertRaisesRegex(PreflightError, "strictly ascending"):
                preflight(run, SPEC, REPO_ROOT)

    def test_a_wrong_schema_fails(self) -> None:
        spec = load_spec()
        spec["schema"] = "quantrule.model-spec/v2"
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            path = write_json(directory, "spec.json", spec)
            with self.assertRaisesRegex(PreflightError, "schema is not"):
                preflight(CONFIG, path, REPO_ROOT)

    def test_the_render_is_deterministic(self) -> None:
        first = spec_preflight.render(preflight(CONFIG, SPEC, REPO_ROOT))
        second = spec_preflight.render(preflight(CONFIG, SPEC, REPO_ROOT))
        self.assertEqual(first, second)
        self.assertTrue(first.startswith("spec_preflight: ok\n"))


if __name__ == "__main__":
    unittest.main()
