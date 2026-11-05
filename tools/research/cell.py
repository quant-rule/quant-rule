#!/usr/bin/env python3
"""Run one pre-registered research cell and write one result.

The cell command receives {config} and {out} substitutions. The runner records
the exact command, exit code, streams, artifact hashes, parsed metrics, verdict,
and limitations. It never opens validation unless the program allows one-shot
access.

Usage:
    python3 tools/research/cell.py \
        --program config/research/<id>/program.json \
        --cell CELL-01 \
        --phase dev \
        --out data/research/<id>/CELL-01
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import adapters
import program as program_module

SCHEMA = "quantrule.research-cell-result/v1"
STREAM_LIMIT = 8_000


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifacts(out_dir: Path) -> list[dict[str, str]]:
    values = []
    for path in sorted(item for item in out_dir.rglob("*") if item.is_file()):
        if path.name == "cell-result.json":
            continue
        values.append(
            {
                "path": str(path.relative_to(out_dir)),
                "sha256": sha256_file(path),
            }
        )
    return values


def _tail(value: str) -> str:
    if len(value) <= STREAM_LIMIT:
        return value
    return value[-STREAM_LIMIT:]


def run_cell(
    program: program_module.Program,
    cell: program_module.Cell,
    phase: str,
    out_dir: Path,
    timeout: int = 3_600,
) -> dict[str, Any]:
    """Run one cell and return its result record."""
    program_module.require_phase(program, phase)
    out_dir = Path(out_dir).expanduser()
    out_dir.mkdir(parents=True, exist_ok=True)
    command = [
        token.replace("{config}", cell.config_dir)
        .replace("{out}", str(out_dir))
        .replace("{phase}", phase)
        for token in cell.command
    ]
    try:
        completed = subprocess.run(
            command,
            cwd=program.repo_root,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
        exit_code: int | None = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        exit_code = None
        stdout = error.stdout or ""
        stderr = f"timeout after {timeout}s"
        if isinstance(stdout, bytes):
            stdout = stdout.decode(errors="replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode(errors="replace")

    limitations: list[str] = []
    metrics: dict[str, Any] = {}
    verdict: str | None = None
    try:
        parsed = adapters.parse(cell.result_parser, out_dir)
        metrics = parsed.metrics
        verdict = parsed.verdict
        limitations.extend(parsed.limitations)
    except (FileNotFoundError, ValueError) as error:
        limitations.append(f"result parse failed: {error}")

    if verdict is None:
        verdict = "pass" if exit_code == 0 else "fail"
    if verdict not in {"pass", "fail"}:
        verdict = "fail"

    record = {
        "schema": SCHEMA,
        "program_id": program.program_id,
        "cell_id": cell.cell_id,
        "hypothesis": cell.hypothesis,
        "phase": phase,
        "command": command,
        "exit_code": exit_code,
        "stdout": _tail(stdout),
        "stderr": _tail(stderr),
        "artifacts": artifacts(out_dir),
        "metrics": metrics,
        "spec_id": metrics.get("spec_id"),
        "prediction_digest": metrics.get("prediction_digest"),
        "verdict": verdict,
        "limitations": limitations,
    }
    (out_dir / "cell-result.json").write_text(
        json.dumps(record, indent=2) + "\n", encoding="utf-8"
    )
    return record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--program", required=True)
    parser.add_argument("--cell", required=True)
    parser.add_argument("--phase", default="dev")
    parser.add_argument("--out", required=True)
    parser.add_argument("--timeout", type=int, default=3_600)
    args = parser.parse_args(argv)
    try:
        loaded = program_module.load_program(args.program)
        cell = loaded.cell(args.cell)
        record = run_cell(loaded, cell, args.phase, Path(args.out), args.timeout)
    except (program_module.ProgramError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"ok {record['cell_id']} {record['verdict']} -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
