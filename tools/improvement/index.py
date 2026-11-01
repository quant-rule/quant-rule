#!/usr/bin/env python3
"""Build a deterministic index of improvement runs under a store root.

The scanner reads ``<root>/<YYYY-MM-DD>/<run_id>/manifest.json`` only. It never
edits a manifest. Rows are ordered by date, then tier (model, strategy,
portfolio), then run id, and both index files are written from that one order so
a rerun over the same tree produces byte-identical output.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

from common import (
    INDEX_SCHEMA,
    RUN_ID_RE,
    TIERS,
    TIER_RANK,
    ToolError,
    compact_json,
    read_json,
    validate_manifest,
    write_json,
)

DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
VERDICT_RE = re.compile(
    r"^\s*(?:[-*]\s*)?(?:\*\*)?[Vv]erdict(?:\*\*)?\s*:\s*(.*?)\s*$"
)
COLUMNS = (
    "run id",
    "date",
    "tier",
    "hypothesis",
    "changed",
    "frozen",
    "baseline",
    "partition",
    "verdict",
    "artifacts",
)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="index.py",
        description=(
            "Index improvement runs under <root>/<YYYY-MM-DD>/<run_id>/ and "
            "write index.json and index.md. The scan is read-only against the "
            "run directories."
        ),
        epilog="Example: python3 tools/improvement/index.py --root data/improvement",
    )
    parser.add_argument("--root", required=True, help="improvement store root")
    return parser.parse_args(argv)


def baseline_cell(value: Any) -> str:
    """Render the optional baseline as one stable cell."""
    if value is None:
        return ""
    if isinstance(value, str):
        return value.strip()
    if isinstance(value, dict) and isinstance(value.get("run_id"), str):
        return value["run_id"].strip()
    return compact_json(value)


def verdict_from_files(run_dir: Path) -> str:
    """Read a verdict from results.json, then decision.md, else pending."""
    results_path = run_dir / "results.json"
    if results_path.is_file():
        results = read_json(results_path)
        if isinstance(results, dict):
            verdict = results.get("verdict")
            if isinstance(verdict, str) and verdict.strip():
                return verdict.strip()
            if results.get("ok") is False:
                return "failed"

    decision_path = run_dir / "decision.md"
    if decision_path.is_file():
        try:
            text = decision_path.read_text(encoding="utf-8")
        except OSError as error:
            raise ToolError(f"cannot read {decision_path}: {error}") from None
        for line in text.splitlines():
            match = VERDICT_RE.match(line)
            if match is None:
                continue
            verdict = match.group(1).strip().strip("*`_ ")
            if verdict:
                return verdict
            break
    return "pending"


def artifact_count(run_dir: Path) -> int:
    """Count regular files under the run's artifacts directory."""
    artifacts = run_dir / "artifacts"
    if not artifacts.is_dir():
        return 0
    return sum(1 for path in artifacts.rglob("*") if path.is_file())


def scan(root: Path) -> list[dict[str, Any]]:
    """Collect one row per manifest, validating it before it enters the index."""
    if not root.is_dir():
        raise ToolError(f"improvement root not found: {root}")

    rows: list[dict[str, Any]] = []
    date_dirs = sorted(
        path
        for path in root.iterdir()
        if path.is_dir() and DATE_RE.fullmatch(path.name)
    )
    for date_dir in date_dirs:
        run_dirs = sorted(path for path in date_dir.iterdir() if path.is_dir())
        for run_dir in run_dirs:
            manifest_path = run_dir / "manifest.json"
            if not manifest_path.is_file():
                continue
            manifest = read_json(manifest_path)
            errors = validate_manifest(manifest)
            if errors:
                detail = "; ".join(errors)
                raise ToolError(f"{manifest_path}: invalid manifest: {detail}")

            run_id = manifest["run_id"]
            run_date = RUN_ID_RE.fullmatch(run_id).group(2)
            if run_date != date_dir.name:
                raise ToolError(
                    f"{manifest_path}: run_id date {run_date} does not match "
                    f"directory {date_dir.name}"
                )
            rows.append(
                {
                    "run_id": run_id,
                    "date": date_dir.name,
                    "tier": manifest["tier"],
                    "hypothesis": " ".join(str(manifest["hypothesis"]).split()),
                    "changed": manifest["changed"],
                    "frozen": ", ".join(manifest["frozen"]),
                    "baseline": baseline_cell(manifest.get("baseline")),
                    "partition": manifest["partition"],
                    "verdict": verdict_from_files(run_dir),
                    "artifacts": artifact_count(run_dir),
                }
            )

    rows.sort(
        key=lambda row: (
            row["date"],
            TIER_RANK.get(row["tier"], len(TIERS)),
            row["run_id"],
        )
    )
    return rows


def markdown_cell(value: Any) -> str:
    text = "" if value is None else str(value)
    text = " ".join(text.split())
    return text.replace("|", "\\|")


def render_markdown(rows: list[dict[str, Any]]) -> str:
    lines = [
        "# Improvement run index",
        "",
        f"Runs: {len(rows)}.",
        "",
        "| " + " | ".join(COLUMNS) + " |",
        "|" + "|".join("---" for _ in COLUMNS) + "|",
    ]
    for row in rows:
        values = (
            row["run_id"],
            row["date"],
            row["tier"],
            row["hypothesis"],
            row["changed"],
            row["frozen"],
            row["baseline"],
            row["partition"],
            row["verdict"],
            row["artifacts"],
        )
        lines.append("| " + " | ".join(markdown_cell(value) for value in values) + " |")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    root = Path(args.root).expanduser().resolve()
    try:
        rows = scan(root)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    document = {
        "schema": INDEX_SCHEMA,
        "count": len(rows),
        "runs": rows,
    }
    try:
        write_json(root / "index.json", document)
        (root / "index.md").write_bytes(render_markdown(rows).encode("utf-8"))
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    except OSError as error:
        print(f"error: cannot write {root / 'index.md'}: {error}", file=sys.stderr)
        return 1

    print(f"indexed {len(rows)} run(s) from {root}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
