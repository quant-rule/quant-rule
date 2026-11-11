#!/usr/bin/env python3
"""Regenerate the simulation store into the external local data root.

The script copies the source and the human marks once, writes the per-year M1.3
report configs with absolute paths, runs the five reports, then runs SIM-01..09
and their checks. SIM-10 is recorded as blocked. It never writes into a task
worktree's own ``data/``.

Usage:
    python3 tools/simulation/reproduce.py --data-root "$HOME/quant-rule-data"
    python3 tools/simulation/reproduce.py --data-root "$HOME/quant-rule-data" --reports-only
    python3 tools/simulation/reproduce.py --data-root "$HOME/quant-rule-data" --runs-only
    python3 tools/simulation/reproduce.py --data-root "$HOME/quant-rule-data" --dry-run
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from data_root import main_worktree, resolve_root

ROOT = Path(__file__).resolve().parents[2]
EXAMPLES = ROOT / "tools" / "simulation" / "examples"
M13_EXAMPLES = EXAMPLES / "m13-2018-2022"
RUN_EXAMPLES = EXAMPLES / "runs"
DATA_VERSION = "binance-spot-btcusdt-m1-2018-01-01_2022-12-31-v1"
SOURCE_NAME = "BTCUSDT_M1_2018-01-01_2025-12-31.csv"
MARK_NAME = "annotations-binance.csv"
RUN_IDS = {
    "SIM-01": "SIM-01-meta-label",
    "SIM-02": "SIM-02-level-rejection",
    "SIM-03": "SIM-03-exit-lab",
    "SIM-04": "SIM-04-failed-breakout",
    "SIM-05": "SIM-05-retest-hold",
    "SIM-06": "SIM-06-trend-pullback",
    "SIM-07": "SIM-07-compression-thrust",
    "SIM-08": "SIM-08-divergence",
    "SIM-09": "SIM-09-climax",
    "SIM-10": "SIM-10-opening-range-blocked",
}


class ReproduceError(Exception):
    """The store cannot be regenerated."""


def _substitute(value: Any, root: Path) -> Any:
    if isinstance(value, str):
        return value.replace("{data_root}", str(root))
    if isinstance(value, list):
        return [_substitute(item, root) for item in value]
    if isinstance(value, dict):
        return {key: _substitute(item, root) for key, item in value.items()}
    return value


def _print_command(command: list[str], dry: bool) -> None:
    print("  " + " ".join(command))
    if not dry:
        subprocess.run(command, check=True)


def copy_sources(root: Path, dry: bool) -> None:
    """Copy the source candles and human marks once into the external root."""
    source_dir = root / "sources"
    source_dir.mkdir(parents=True, exist_ok=True)
    main = main_worktree()
    source = main / "data" / "sources" / SOURCE_NAME
    marks = main / "data" / "improvement" / "2026-11-09" / "BINANCE-BTCUSDT" / MARK_NAME
    for item in (source, marks):
        target = source_dir / item.name
        if target.exists():
            print(f"source present: {target}")
            continue
        if not item.is_file():
            raise ReproduceError(f"missing local source {item}")
        print(f"copy {item} -> {target}")
        if not dry:
            shutil.copy2(item, target)


def write_report_configs(root: Path, dry: bool) -> Path:
    """Copy the M1.3 configs with absolute data-root paths."""
    config_dir = root / "simulation" / "_data" / "M1.3-2018-2022" / "config"
    print(f"report configs: {config_dir}")
    if dry:
        return config_dir
    config_dir.mkdir(parents=True, exist_ok=True)
    for name in ("model.json", "policy.json", "execution.json", "portfolio.json"):
        shutil.copy2(M13_EXAMPLES / name, config_dir / name)
    for year in range(2018, 2023):
        for prefix in ("data", "run"):
            source = M13_EXAMPLES / f"{prefix}-{year}.json"
            value = _substitute(json.loads(source.read_text(encoding="utf-8")), root)
            (config_dir / source.name).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    return config_dir


def run_reports(root: Path, binary: Path, force: bool, dry: bool) -> None:
    """Run the five per-year M1.3 reports."""
    config_dir = write_report_configs(root, dry)
    if not dry and not binary.is_file():
        raise ReproduceError(f"report binary not found at {binary}; build it with cargo build --release")
    for year in range(2018, 2023):
        out = root / "simulation" / "_data" / "M1.3-2018-2022" / f"report-{year}"
        if out.exists() and not force:
            print(f"report present: {out}")
            continue
        if force and out.exists() and not dry:
            shutil.rmtree(out)
        print(f"report {year}: {out}")
        _print_command(
            [
                str(binary),
                "report",
                "model",
                "--config",
                str(config_dir / f"run-{year}.json"),
                "--out",
                str(out),
            ],
            dry,
        )


def _run_dir(root: Path, run_id: str) -> Path:
    return root / "simulation" / "2026-11-10" / RUN_IDS[run_id]


def run_simulations(root: Path, force: bool, dry: bool) -> None:
    """Run SIM-01..09 and their checks; record SIM-10 as blocked."""
    sim_dir = root / "simulation" / "2026-11-10"
    if not dry:
        sim_dir.mkdir(parents=True, exist_ok=True)
    base = [
        sys.executable,
        str(ROOT / "tools" / "simulation" / "run.py"),
    ]

    # SIM-01 meta-label.
    out = _run_dir(root, "SIM-01")
    print(f"SIM-01: {out}")
    _print_command(
        [
            sys.executable,
            str(ROOT / "tools" / "simulation" / "meta.py"),
            "--meta-config",
            str(RUN_EXAMPLES / "SIM-01.meta.json"),
            "--out",
            str(out),
            "--data-root",
            str(root),
        ],
        dry,
    )

    # SIM-02 first: SIM-03 consumes its marks.
    for run_id in ("SIM-02", "SIM-04", "SIM-05", "SIM-06", "SIM-07", "SIM-08", "SIM-09"):
        out = _run_dir(root, run_id)
        config = RUN_EXAMPLES / f"{run_id}.config.json"
        if out.exists() and not force:
            print(f"{run_id} present: {out}")
        else:
            print(f"{run_id}: {out}")
            _print_command(
                base
                + [
                    "--run-config",
                    str(config),
                    "--out",
                    str(out),
                    "--data-root",
                    str(root),
                    "--force",
                ],
                dry,
            )

    # SIM-03 exit lab.
    out = _run_dir(root, "SIM-03")
    print(f"SIM-03: {out}")
    _print_command(
        [
            sys.executable,
            str(ROOT / "tools" / "simulation" / "exit_lab.py"),
            "--exit-config",
            str(RUN_EXAMPLES / "SIM-03.exit.json"),
            "--out",
            str(out),
            "--data-root",
            str(root),
        ],
        dry,
    )

    # Checks.
    checks = [
        ("SIM-02.check.json", _run_dir(root, "SIM-02")),
        ("SIM-04.check.json", _run_dir(root, "SIM-04")),
        ("SIM-05.check.json", _run_dir(root, "SIM-05")),
        ("SIM-06.check.json", _run_dir(root, "SIM-06")),
        ("SIM-07.check.json", _run_dir(root, "SIM-07")),
        ("SIM-08.bull.check.json", _run_dir(root, "SIM-08") / "check-bull-bars"),
        ("SIM-08.bear.check.json", _run_dir(root, "SIM-08") / "check-bear"),
        ("SIM-09.climax.check.json", _run_dir(root, "SIM-09") / "check-climax"),
        ("SIM-09.absorption.check.json", _run_dir(root, "SIM-09") / "check-absorption"),
    ]
    for name, out in checks:
        print(f"check {name}: {out}")
        _print_command(
            [
                sys.executable,
                str(ROOT / "tools" / "simulation" / "check.py"),
                "--check-config",
                str(RUN_EXAMPLES / name),
                "--out",
                str(out),
                "--data-root",
                str(root),
            ],
            dry,
        )

    # SIM-10 blocked record.
    blocked = _run_dir(root, "SIM-10")
    print(f"SIM-10: blocked record at {blocked}")
    if not dry:
        blocked.mkdir(parents=True, exist_ok=True)
        reason = (RUN_EXAMPLES / "SIM-10.blocked.md").read_text(encoding="utf-8")
        (blocked / "decision.md").write_text(reason, encoding="utf-8")
        (blocked / "manifest.json").write_text(
            json.dumps(
                {
                    "schema": "quantrule.simulation-run/v1",
                    "run_id": "SIM-10",
                    "kind": "blocked",
                    "model": "M1.3",
                    "registry_version": "2026-11-10.2",
                    "data_version": DATA_VERSION,
                    "reason": "no session block; signal.opening_range_sweep is absent",
                    "holdout_access": "none",
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", type=Path)
    parser.add_argument("--binary", type=Path, help="quantrule release binary")
    parser.add_argument("--reports-only", action="store_true")
    parser.add_argument("--runs-only", action="store_true")
    parser.add_argument("--force", action="store_true", help="replace existing outputs")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv[1:])
    root = resolve_root(args.data_root)
    binary = args.binary or (main_worktree() / "target" / "release" / "quantrule")
    print(f"data root: {root}")
    print(f"binary: {binary}")
    try:
        if not args.runs_only:
            copy_sources(root, args.dry_run)
            run_reports(root, binary, args.force, args.dry_run)
        if not args.reports_only:
            run_simulations(root, args.force, args.dry_run)
    except (ReproduceError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
