#!/usr/bin/env python3
"""Run one pre-registered M2.x model cell.

A cell owns four instrument specs and one run config per instrument and year.
The runner executes `quantrule report model` for each instrument-year, merges
the yearly prediction and readings bundles per instrument, then runs one pooled
`model_prediction` evaluation over the four merged bundles. It writes the
standard model artifacts under `<out>/<instrument>/<year>/model/`, the merged
bundle under `<out>/<instrument>/model/`, the pooled evaluation under
`<out>/evaluation/`, and the root `model_prediction.json`/`.md` that the research
cell adapter reads.

The runner is deterministic: the instrument and year order are fixed, the
evaluator seeds are explicit, and the prediction digest is a SHA-256 over the
sorted per-instrument prediction files.

Usage:
    python3 tools/research/model_cell.py \
        --cell-dir config/research/PROGRAM-2026-11-05-03/cells/M2.1 \
        --phase dev \
        --out data/research/PROGRAM-2026-11-05-03/dev/M2.1
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

INSTRUMENTS = ["XAU", "BTC", "SP500", "EURUSD"]
PARTITIONS = {
    "dev": ("model_dev", "2018-01-01", "2020-12-31", (2018, 2019, 2020)),
    "validation": ("model_validation", "2021-01-01", "2022-12-31", (2021, 2022)),
}
MERGE_SCHEMA = "quantrule.model-cell-merge/v1"


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cell-dir", required=True)
    parser.add_argument("--phase", choices=sorted(PARTITIONS), default="dev")
    parser.add_argument("--out", required=True)
    parser.add_argument("--runs-dir", default=None)
    parser.add_argument("--qr", default=None, help="quantrule binary")
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument("--evaluator", default="tools/improvement/model_prediction.py")
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--horizon", type=int, default=20)
    parser.add_argument("--target-atr", type=float, default=2.0)
    parser.add_argument("--stop-atr", type=float, default=1.0)
    parser.add_argument("--cost-bps", type=float, default=6.0)
    parser.add_argument("--primary-cells", type=int, default=5)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--timeout", type=int, default=3_600)
    return parser.parse_args(argv)


def shell_binary(root: Path, requested: str | None) -> Path:
    if requested:
        path = Path(requested).expanduser()
        if not path.is_absolute():
            path = root / path
        if not path.is_file():
            raise SystemExit(f"error: quantrule binary not found: {path}")
        return path
    for candidate in ("target/release/quantrule", "target/debug/quantrule"):
        path = root / candidate
        if path.is_file():
            return path
    raise SystemExit(
        "error: no quantrule binary found; build one or pass --qr target/release/quantrule"
    )


def run_command(
    command: list[str], root: Path, timeout: int, label: str
) -> None:
    try:
        completed = subprocess.run(
            command,
            cwd=root,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        raise SystemExit(f"error: {label} timed out after {timeout}s") from None
    except OSError as error:
        raise SystemExit(f"error: cannot start {label}: {error}") from None
    if completed.returncode != 0:
        message = completed.stderr.strip() or completed.stdout.strip()
        raise SystemExit(f"error: {label} failed ({completed.returncode}): {message}")


def read_csv_rows(path: Path) -> tuple[list[str], list[list[str]]]:
    try:
        with path.open(newline="", encoding="utf-8") as handle:
            reader = csv.reader(handle)
            header = next(reader, None)
            rows = list(reader)
    except OSError as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None
    if header is None:
        raise SystemExit(f"error: {path} has no header")
    return header, rows


def write_csv_rows(path: Path, header: list[str], rows: list[list[str]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        writer.writerows(rows)


def file_digest(path: Path) -> str:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def merge_bundles(destination: Path, sources: list[tuple[int, Path]]) -> dict[str, Any]:
    """Merge yearly bundles into one continuous per-instrument bundle.

    Bars are reindexed per timeframe over the merged readings, and predictions
    are remapped by timestamp. A duplicate timestamp is a fold overlap and is
    rejected rather than silently averaged or dropped.
    """
    destination.mkdir(parents=True, exist_ok=True)
    readings_header, readings_rows = merge_rows(sources, "readings.csv")
    readings_rows.sort(key=lambda row: (int(row[0]), int(row[2])))
    bar_by_time: dict[tuple[int, int], int] = {}
    previous_timeframe: int | None = None
    bar = 0
    for row in readings_rows:
        timeframe = int(row[0])
        timestamp = int(row[2])
        if timeframe != previous_timeframe:
            previous_timeframe = timeframe
            bar = 0
        key = (timeframe, timestamp)
        if key in bar_by_time:
            raise SystemExit(
                f"error: duplicate readings timestamp {timestamp} for timeframe {timeframe}"
            )
        bar_by_time[key] = bar
        row[1] = str(bar)
        bar += 1
    prediction_header, prediction_rows = merge_rows(sources, "prediction.csv")
    prediction_rows.sort(key=lambda row: (int(row[0]), int(row[2])))
    for row in prediction_rows:
        key = (int(row[0]), int(row[2]))
        merged_bar = bar_by_time.get(key)
        if merged_bar is None:
            raise SystemExit(
                f"error: prediction timestamp {key[1]} has no readings row "
                f"for timeframe {key[0]}"
            )
        row[1] = str(merged_bar)
    write_csv_rows(destination / "readings.csv", readings_header, readings_rows)
    write_csv_rows(destination / "prediction.csv", prediction_header, prediction_rows)
    manifest = {
        "schema": MERGE_SCHEMA,
        "years": [year for year, _ in sources],
        "readings_rows": len(readings_rows),
        "prediction_rows": len(prediction_rows),
        "sources": [
            {
                "year": year,
                "prediction_sha256": file_digest(bundle / "prediction.csv"),
                "readings_sha256": file_digest(bundle / "readings.csv"),
            }
            for year, bundle in sources
        ],
    }
    (destination / "merge.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return manifest


def merge_rows(
    sources: list[tuple[int, Path]], filename: str
) -> tuple[list[str], list[list[str]]]:
    header: list[str] | None = None
    rows: list[list[str]] = []
    for _, bundle in sources:
        file_header, file_rows = read_csv_rows(bundle / filename)
        if header is None:
            header = file_header
        elif file_header != header:
            raise SystemExit(f"error: {filename} headers differ across years")
        rows.extend(file_rows)
    if header is None:
        raise SystemExit(f"error: no {filename} rows to merge")
    return header, rows


def prediction_digest(bundles: list[tuple[str, Path]]) -> str:
    digest = hashlib.sha256()
    for name, directory in sorted(bundles):
        path = directory / "prediction.csv"
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_digest(path).encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def write_root_evaluation(
    out: Path,
    document: dict[str, Any],
    digest: str,
    phase: str,
    years: tuple[int, ...],
) -> None:
    document["phase"] = phase
    document["prediction_digest"] = digest
    document["years"] = list(years)
    (out / "model_prediction.json").write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    source = out / "evaluation" / "model_prediction.md"
    if source.is_file():
        shutil.copy2(source, out / "model_prediction.md")


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    root = Path(args.repo_root).expanduser().resolve()
    cell_dir = Path(args.cell_dir).expanduser()
    if not cell_dir.is_absolute():
        cell_dir = root / cell_dir
    out = Path(args.out).expanduser()
    if not out.is_absolute():
        out = root / out
    out.mkdir(parents=True, exist_ok=True)
    runs_dir = (
        Path(args.runs_dir).expanduser()
        if args.runs_dir
        else cell_dir.parents[1] / "runs"
    )
    if not runs_dir.is_absolute():
        runs_dir = root / runs_dir
    partition, split_start, split_end, years = PARTITIONS[args.phase]
    qr = shell_binary(root, args.qr)
    cell_id = cell_dir.name
    bundles: list[tuple[str, Path]] = []
    for instrument in INSTRUMENTS:
        spec = cell_dir / f"spec-{instrument}.json"
        if not spec.is_file():
            raise SystemExit(f"error: spec not found: {spec}")
        yearly: list[tuple[int, Path]] = []
        for year in years:
            run = runs_dir / instrument / str(year) / "run.json"
            if not run.is_file():
                raise SystemExit(f"error: run config not found: {run}")
            year_out = out / instrument / str(year)
            run_command(
                [
                    str(qr),
                    "report",
                    "model",
                    "--config",
                    str(run),
                    "--spec",
                    str(spec),
                    "--out",
                    str(year_out),
                ],
                root,
                args.timeout,
                f"{cell_id} {instrument} {year} model",
            )
            bundle = year_out / "model"
            if not (bundle / "prediction.csv").is_file():
                raise SystemExit(f"error: prediction.csv not found under {bundle}")
            yearly.append((year, bundle))
        merged = out / instrument / "model"
        merge_bundles(merged, yearly)
        bundles.append((instrument, merged))
    digest = prediction_digest(bundles)
    evaluator = Path(args.evaluator).expanduser()
    if not evaluator.is_absolute():
        evaluator = root / evaluator
    evaluation = out / "evaluation"
    command = [
        args.python,
        str(evaluator),
        "--spec-id",
        cell_id,
        "--family",
        "M2",
        "--partition",
        partition,
        "--split-start",
        split_start,
        "--split-end",
        split_end,
        "--horizon",
        str(args.horizon),
        "--target-atr",
        str(args.target_atr),
        "--stop-atr",
        str(args.stop_atr),
        "--cost-bps",
        str(args.cost_bps),
        "--primary-cells",
        str(args.primary_cells),
        "--seed",
        str(args.seed),
        "--out",
        str(evaluation),
    ]
    for instrument, bundle in bundles:
        command.extend(["--bundle", f"{instrument}={bundle}"])
    run_command(command, root, args.timeout, f"{cell_id} evaluation")
    document = json.loads(
        (evaluation / "model_prediction.json").read_text(encoding="utf-8")
    )
    write_root_evaluation(out, document, digest, args.phase, years)
    print(
        f"ok {cell_id} phase={args.phase} instruments={len(bundles)} "
        f"years={len(years)} grade={document.get('grade')} "
        f"verdict={document.get('verdict')} digest={digest[:12]}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
