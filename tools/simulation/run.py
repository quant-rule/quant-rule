#!/usr/bin/env python3
"""Run one simulation pass: select, render, label, and review.

The orchestrator is mechanical. It loads a recorded M1.3 bundle, selects windows,
renders clean charts, proposes hindsight marks, validates them, writes the review,
and records the manifest. It does not decide that a pattern is an edge; the AI
visual review and the human decision happen after this command.

Usage:
    python3 tools/simulation/run.py \
      --run-config data/simulation/2026-11-10/SIM-02/config.json \
      --out data/simulation/2026-11-10/SIM-02
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from bundle import (
    BundleError,
    load_bundles,
    load_catalog,
    parse_utc,
    validate_m13,
)
from data_root import (
    DataRootError,
    current_worktree,
    guard_root,
    is_inside,
    resolve_root,
    substitute,
)
from marks import MarkError, read_marks, validate_marks, write_marks
from oracle import OracleSpec, oracle_for_window
from render import LIGHT, DARK, RenderError, contact_sheet, render_windows
from review import build_review
from window_select import SelectError, Window, WindowSpec, select_windows, write_windows

DEFAULT_REGISTRY = Path("docs/registry/indicators.json")
VALID_STUDIES = {"ema", "rsi", "rvol", "adx"}
PANEL_STUDIES = {"rsi", "rvol", "adx"}
REQUIRED_CONFIG = {"schema", "run_id", "premise", "timeframe", "from", "to", "events"}


class RunError(Exception):
    """The run configuration or one of its stages is invalid."""


def sha256_file(path: Path) -> str:
    """Return the SHA-256 of a file."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def code_commit(repo_root: Path, override: str | None) -> str:
    """Return the recorded commit, or an explicit unknown marker."""
    if override:
        return override
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return "unknown"
    return result.stdout.strip() or "unknown"


def load_config(path: Path) -> dict[str, Any]:
    """Read and minimally validate a run configuration."""
    config = json.loads(path.read_text(encoding="utf-8"))
    missing = REQUIRED_CONFIG - set(config)
    if missing:
        raise RunError(f"{path} is missing fields {sorted(missing)}")
    if config["schema"] != "quantrule.simulation-run/v1":
        raise RunError(f"{path} has an unknown schema {config['schema']!r}")
    if not str(config["run_id"]).strip():
        raise RunError("run_id must not be empty")
    return config


def resolve_paths(config_path: Path, config: dict[str, Any], data_root: Path) -> tuple[list[Path], Path]:
    """Resolve the bundle list and registry path against the data root."""
    base = config_path.parent
    raw = config.get("bundles", [config.get("bundle")])
    if not raw or any(not item for item in raw):
        raise RunError("provide bundle or a non-empty bundles list")
    bundles = []
    for item in raw:
        value = substitute(item, data_root)
        path = Path(value)
        if not path.is_absolute():
            path = (base / path).resolve()
        bundles.append(path)
    registry = Path(config.get("registry", DEFAULT_REGISTRY))
    if not registry.is_absolute():
        registry = (Path(__file__).resolve().parents[2] / registry).resolve()
    return bundles, registry


def event_codes(config: dict[str, Any], registry_path: Path) -> tuple[tuple[int, ...], dict[int, str]]:
    """Resolve event names or codes against the registry."""
    catalog = load_catalog(registry_path)
    reverse = {name: code for code, name in catalog.items()}
    codes: list[int] = []
    for entry in config["events"]:
        if isinstance(entry, int):
            if entry not in catalog:
                raise RunError(f"event code {entry} is not in the registry")
            codes.append(entry)
            continue
        name = str(entry)
        if name in reverse:
            codes.append(reverse[name])
            continue
        try:
            code = int(name.split(".")[-1])
        except ValueError as error:
            raise RunError(f"unknown event {name!r}") from error
        if code not in catalog:
            raise RunError(f"unknown event {name!r}")
        codes.append(code)
    if not codes:
        raise RunError("events must not be empty")
    return tuple(sorted(set(codes))), catalog


def parse_window(value: str, name: str) -> int:
    """Parse an ISO date or timestamp."""
    try:
        return int(value)
    except ValueError:
        try:
            return parse_utc(value)
        except ValueError as error:
            raise RunError(f"{name} must be an ISO UTC timestamp or Unix seconds") from error


def window_records(selection: dict[str, Any], bundle, catalog: dict[int, str]) -> list[Window]:
    """Rebuild the Window objects from the selected records."""
    windows: list[Window] = []
    for record in selection["windows"]:
        codes = {int(event["code"]) for event in record["events"]}
        events = tuple(
            event for event in bundle.events_at(record["candidate_bar"]) if event.code in codes
        )
        windows.append(
            Window(
                window_id=record["window_id"],
                start_index=record["start_index"],
                end_index=record["end_index"],
                candidate_index=record["candidate_index"],
                events=events,
            )
        )
    return windows


def write_detail(path: Path, rows: list[dict[str, Any]]) -> None:
    """Write the mark detail sidecar."""
    fields = [
        "chart",
        "run_id",
        "window_id",
        "side",
        "quality",
        "entry_bar",
        "entry_index",
        "entry_price",
        "atr",
        "rule_exit_bar",
        "perfect_exit_bar",
        "mfe_atr",
        "mae_atr",
        "mfe_to_rule_atr",
        "mae_to_rule_atr",
        "mfe_before_bad_stop_atr",
        "good",
        "bad",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row.get(field) for field in fields})


def write_reproduce(path: Path, config_path: Path, output: Path, config: dict[str, Any], data_root: Path) -> None:
    """Write the command and the input identity."""
    lines = [
        f"# Reproduce {config['run_id']}",
        "",
        "```bash",
        f"python3 tools/simulation/run.py --run-config {config_path} --out {output}",
        "```",
        "",
        "| Input | Value |",
        "|---|---|",
        f"| Run config | `{config_path}` |",
        f"| Data root | `{data_root}` |",
        f"| Bundles | `{config.get('bundles', [config.get('bundle')])}` |",
        f"| Model | `M1.3` |",
        f"| Registry | `2026-11-10.2` |",
        f"| Seed | `{config.get('seed', 0)}` |",
        "",
    ]
    path.write_text("\n".join(lines), encoding="utf-8")


def write_decision(path: Path, run_id: str) -> None:
    """Write the decision skeleton; the human fills the verdict."""
    lines = [
        f"# Decision — {run_id}",
        "",
        "- Status: pending",
        "- Verdict: pending",
        "- Human acceptance: pending",
        "",
        "## Decision",
        "",
        "Test / refine / stop, with the check evidence.",
        "",
        "## Next action",
        "",
        "",
    ]
    path.write_text("\n".join(lines), encoding="utf-8")


def build(args: argparse.Namespace) -> dict[str, Any]:
    """Execute one run and return its manifest."""
    config_path = Path(args.run_config).resolve()
    data_root = resolve_root(args.data_root)
    guard_root(data_root, args.allow_worktree_data)
    config = load_config(config_path)
    bundle_paths, registry_path = resolve_paths(config_path, config, data_root)
    codes, catalog = event_codes(config, registry_path)
    from_ts = parse_window(str(config["from"]), "from")
    to_ts = parse_window(str(config["to"]), "to")
    timeout = int(config.get("timeframe_secs", 900))
    bundle = load_bundles(
        bundle_paths, timeout, from_ts, to_ts, data_version=config.get("data_version")
    )
    if config.get("strict_m13", True):
        validate_m13(bundle)
    out = Path(args.out).resolve()
    if is_inside(out, current_worktree()) and not args.allow_worktree_data:
        raise RunError(
            f"the output {out} is inside the current worktree; "
            "write to the external data root or pass --allow-worktree-data"
        )
    if out.exists() and (out / "manifest.json").is_file() and not args.force:
        raise RunError(f"{out} already has a manifest; pass --force to replace it")

    window_spec = WindowSpec(
        event_codes=codes,
        window_bars=int(config.get("window_bars", 400)),
        candidate_zone=tuple(config.get("candidate_zone", [80, 120])),
        max_windows=int(config.get("max_windows", 48)),
        min_windows=int(config.get("min_windows", 30)),
        seed=int(config.get("seed", 0)),
        overlap_bars=int(config.get("overlap_bars", 40)),
        from_ts=from_ts,
        to_ts=to_ts,
    )
    selection = select_windows(bundle, window_spec, catalog)
    if selection["selected_count"] == 0:
        raise RunError("no window matched the event filter; choose another event or range")
    windows_path = out / "windows.json"
    write_windows(windows_path, selection)
    windows = window_records(selection, bundle, catalog)

    oracle_spec = OracleSpec(**config.get("oracle", {}))
    oracle_spec.validate()
    marks: list[dict[str, str]] = []
    details: list[dict[str, Any]] = []
    for window in windows:
        chart = f"{config['run_id'].lower()}-{window.window_id}.png"
        proposed, detail = oracle_for_window(
            bundle, window, chart, oracle_spec, str(config["run_id"])
        )
        marks.extend(proposed)
        details.extend(detail)

    studies = tuple(config.get("studies", ["ema"]))
    unknown = set(studies) - VALID_STUDIES
    if unknown:
        raise RunError(f"unknown studies {sorted(unknown)}")
    if len(set(studies) & PANEL_STUDIES) > 1:
        raise RunError("choose at most one indicator panel: rsi, rvol, or adx")
    theme = DARK if config.get("theme") == "dark" else LIGHT
    charts_dir = out / "charts"
    if args.force and charts_dir.exists():
        shutil.rmtree(charts_dir)
    charts, manifest = render_windows(
        bundle,
        selection,
        marks,
        charts_dir,
        str(config["run_id"]),
        catalog,
        studies,
        str(config.get("title", "BTCUSDT 15m")),
        theme,
        zoom=bool(config.get("zoom", True)),
        zoom_bars=int(config.get("zoom_bars", 30)),
    )
    marks = validate_marks(marks, charts)
    marks_path = out / "marks-ai.csv"
    write_marks(marks_path, marks)
    details_path = out / "marks-detail.csv"
    write_detail(details_path, details)
    contact_sheet(charts_dir, manifest, out / "contact-sheet.png")
    build_review(
        bundle,
        charts,
        marks,
        details,
        out / "review.md",
        str(config["run_id"]),
        str(config["premise"]),
        catalog,
        {catalog[code] for code in codes},
    )
    write_reproduce(out / "reproduce.md", config_path, out, config, data_root)
    write_decision(out / "decision.md", str(config["run_id"]))

    manifest_record = {
        "schema": "quantrule.simulation-run/v1",
        "run_id": config["run_id"],
        "premise": config["premise"],
        "model": "M1.3",
        "registry_version": "2026-11-10.2",
        "data_version": bundle.data_version,
        "model_version": bundle.model_version,
        "code_commit": code_commit(Path(__file__).resolve().parents[2], config.get("code_commit")),
        "date": config.get("date", "2026-11-10"),
        "timeframe_secs": timeout,
        "label_window": {"from": config["from"], "to": config["to"]},
        "check_window": config.get("check_window", {"from": "2020-01-01", "to": "2022-12-31"}),
        "holdout_access": "none",
        "data_root": str(data_root),
        "bundle": str(bundle_paths[0]),
        "bundles": [str(path) for path in bundle_paths],
        "seed": window_spec.seed,
        "window_spec": {
            "window_bars": window_spec.window_bars,
            "candidate_zone": list(window_spec.candidate_zone),
            "max_windows": window_spec.max_windows,
            "min_windows": window_spec.min_windows,
            "overlap_bars": window_spec.overlap_bars,
        },
        "oracle_spec": oracle_spec.__dict__,
        "studies": list(studies),
        "zoom": bool(config.get("zoom", True)),
        "zoom_bars": int(config.get("zoom_bars", 30)),
        "event_codes": list(codes),
        "event_names": [catalog[code] for code in codes],
        "selected_count": selection["selected_count"],
        "candidate_count": selection["candidate_count"],
        "underpowered": selection["underpowered"],
        "marks": len(marks),
        "charts": len(charts),
        "hashes": {
            "config": sha256_file(config_path),
            "windows": sha256_file(windows_path),
            "marks": sha256_file(marks_path),
            "chart_manifest": sha256_file(charts_dir / "manifest.json"),
        },
        "visual_review": "pending",
        "check_status": "pending",
    }
    (out / "manifest.json").write_text(
        json.dumps(manifest_record, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return manifest_record


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-config", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--data-root", type=Path, help="persistent local data root")
    parser.add_argument(
        "--allow-worktree-data",
        action="store_true",
        help="allow data inside the current worktree (disposable runs only)",
    )
    parser.add_argument("--force", action="store_true", help="replace an existing run manifest")
    args = parser.parse_args(argv[1:])
    try:
        result = build(args)
    except (RunError, BundleError, SelectError, RenderError, MarkError, DataRootError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"wrote {result['run_id']} to {Path(args.out).resolve()}")
    print(f"windows: {result['selected_count']} selected from {result['candidate_count']}")
    print(f"marks: {result['marks']} in {result['charts']} charts")
    if result["underpowered"]:
        print("warning: underpowered sample; no rule claim")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
