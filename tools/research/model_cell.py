#!/usr/bin/env python3
"""Run one pre-registered M2.x model cell.

A cell owns four instrument specs and four run configs. The runner executes
`quantrule report model` for each instrument, then one pooled
`model_prediction` evaluation over the four bundles. It writes the standard
model artifacts under `<out>/<instrument>/model/`, the pooled evaluation under
`<out>/evaluation/`, and the root `model_prediction.json`/`.md` that the research
cell adapter reads.

The runner is deterministic: the instrument order is fixed, the evaluator seeds
are explicit, and the prediction digest is a SHA-256 over the sorted per-
instrument prediction files.

Usage:
    python3 tools/research/model_cell.py \
        --cell-dir config/research/PROGRAM-2026-11-05-03/cells/M2.1 \
        --phase dev \
        --out data/research/PROGRAM-2026-11-05-03/dev/M2.1
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

INSTRUMENTS = ["XAU", "BTC", "SP500", "EURUSD"]
PARTITIONS = {
    "dev": ("model_dev", "2018-01-01", "2020-12-31"),
    "validation": ("model_validation", "2021-01-01", "2022-12-31"),
}


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


def prediction_digest(bundles: list[tuple[str, Path]]) -> str:
    digest = hashlib.sha256()
    for name, directory in sorted(bundles):
        path = directory / "prediction.csv"
        try:
            content = path.read_bytes()
        except OSError as error:
            raise SystemExit(f"error: cannot read {path}: {error}") from None
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(hashlib.sha256(content).hexdigest().encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def write_root_evaluation(out: Path, document: dict[str, Any], digest: str, phase: str) -> None:
    document["phase"] = phase
    document["prediction_digest"] = digest
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
    partition, split_start, split_end = PARTITIONS[args.phase]
    qr = shell_binary(root, args.qr)
    cell_id = cell_dir.name
    bundles: list[tuple[str, Path]] = []
    for instrument in INSTRUMENTS:
        spec = cell_dir / f"spec-{instrument}.json"
        run = runs_dir / instrument / "run.json"
        if not spec.is_file():
            raise SystemExit(f"error: spec not found: {spec}")
        if not run.is_file():
            raise SystemExit(f"error: run config not found: {run}")
        instrument_out = out / instrument
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
                str(instrument_out),
            ],
            root,
            args.timeout,
            f"{cell_id} {instrument} model",
        )
        bundle = instrument_out / "model"
        if not (bundle / "prediction.csv").is_file():
            raise SystemExit(f"error: prediction.csv not found under {bundle}")
        bundles.append((instrument, bundle))
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
    write_root_evaluation(out, document, digest, args.phase)
    print(
        f"ok {cell_id} phase={args.phase} instruments={len(bundles)} "
        f"grade={document.get('grade')} verdict={document.get('verdict')} "
        f"digest={digest[:12]}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
