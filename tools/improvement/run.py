#!/usr/bin/env python3
"""Validate and execute one improvement run manifest.

The manifest is the contract. This command refuses a non-empty output root,
copies the manifest and its configuration for provenance, runs the shell once
for every tier with the manifest's run config, and records the observed command,
exit code, captured streams, parsed JSON, and artifact digests. A non-zero shell
exit is a failure, not a partially successful run.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path
from typing import Any

from common import (
    CONFIG_SCHEMA,
    IMPROVEMENT_DIR,
    REPO_ROOT,
    RESULT_SCHEMA,
    RUN_ID_RE,
    ToolError,
    compact_json,
    file_inventory,
    is_within,
    load_manifest,
    repo_relative,
    resolve_shell,
    run_process,
    write_json,
)

DEFAULT_SHELL = "target/debug/quantrule"
INDEX_ROOT = REPO_ROOT / "data" / "improvement"


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="run.py",
        description=(
            "Validate a manifest, create its run folder, execute the shell "
            "backtest, write results.json, and refresh the store index. The "
            "command refuses a non-empty run folder and never treats a non-zero "
            "shell exit as success."
        ),
        epilog=(
            "Example: python3 tools/improvement/run.py "
            "data/improvement/2026-11-01/MODEL-2026-11-01-01/manifest.json "
            "--qr target/debug/quantrule"
        ),
    )
    parser.add_argument("manifest", help="path to manifest.json")
    parser.add_argument(
        "--qr",
        default=DEFAULT_SHELL,
        help=f"shell binary (default: {DEFAULT_SHELL})",
    )
    return parser.parse_args(argv)


def run_date(manifest: dict[str, Any]) -> str:
    match = RUN_ID_RE.fullmatch(manifest["run_id"])
    if match is None:
        raise ToolError(f"run_id is not valid: {manifest['run_id']!r}")
    return match.group(2)


def output_root(manifest: dict[str, Any]) -> Path:
    """Resolve the run directory, defaulting to the canonical store layout."""
    configured = manifest.get("output_root")
    if configured is None:
        return (INDEX_ROOT / run_date(manifest) / manifest["run_id"]).resolve()
    path = Path(configured).expanduser()
    if not path.is_absolute():
        path = REPO_ROOT / path
    return path.resolve()


def ensure_empty_root(root: Path) -> None:
    if not root.exists():
        return
    if not root.is_dir():
        raise ToolError(f"output root exists and is not a directory: {root}")
    if any(root.iterdir()):
        raise ToolError(f"output root is not empty: {root}")


def resolve_run_config(manifest_path: Path, value: str) -> Path:
    path = Path(value).expanduser()
    candidates = (
        [path]
        if path.is_absolute()
        else [manifest_path.parent / path, REPO_ROOT / path]
    )
    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()
    tried = ", ".join(str(candidate) for candidate in candidates)
    raise ToolError(f"run config not found: {value!r} (tried {tried})")


def sibling_configs(config_path: Path) -> list[Path]:
    siblings = [
        path
        for path in config_path.parent.iterdir()
        if path.is_file() and path.suffix.lower() == ".json"
    ]
    resolved = {path.resolve() for path in siblings}
    if config_path.resolve() not in resolved:
        siblings.append(config_path)
    return sorted(siblings, key=lambda path: path.name)


def prepare_config(config_path: Path, config_dir: Path) -> None:
    """Copy the run config and sibling JSON files, or record outside paths."""
    config_dir.mkdir(parents=True, exist_ok=True)
    sources: list[dict[str, Any]] = []
    copied = is_within(config_path, REPO_ROOT)
    if copied:
        for source in sibling_configs(config_path):
            destination = config_dir / source.name
            try:
                shutil.copyfile(source, destination)
            except OSError as error:
                raise ToolError(f"cannot copy {source} to {destination}: {error}") from None
            sources.append({"source": repo_relative(source), "dest": source.name})
    else:
        for source in sibling_configs(config_path):
            sources.append({"source": str(source.resolve()), "dest": None})

    write_json(
        config_dir / "sources.json",
        {
            "schema": CONFIG_SCHEMA,
            "run_config": repo_relative(config_path),
            "config_dir": repo_relative(config_path.parent),
            "copied": copied,
            "files": sources,
        },
    )


def baseline_label(manifest: dict[str, Any]) -> str:
    baseline = manifest.get("baseline")
    if isinstance(baseline, str):
        return baseline
    if baseline is not None:
        return compact_json(baseline)
    return "(none)"


def review_text(manifest: dict[str, Any]) -> str:
    frozen = ", ".join(manifest["frozen"]) if manifest["frozen"] else "(none)"
    return (
        f"# Review: {manifest['run_id']}\n"
        "\n"
        f"- Tier: {manifest['tier']}\n"
        f"- Changed: {manifest['changed']}\n"
        f"- Frozen: {frozen}\n"
        f"- Partition: {manifest['partition']}\n"
        f"- Hypothesis: {manifest['hypothesis']}\n"
        f"- Green criteria: `{compact_json(manifest['green_criteria'])}`\n"
        "- Reviewer: Result Analyst\n"
        "- Date: <YYYY-MM-DD> (UTC)\n"
        "\n"
        "## Evidence\n"
        "\n"
        "- `results.json` records the command, exit code, captured streams, and "
        "artifact digests.\n"
        "- `artifacts/evidence.json` records the published numbers extracted from "
        "the report; run `python3 tools/improvement/evidence.py <run-dir>`.\n"
        f"- Baseline: {baseline_label(manifest)}\n"
        "- Rerun check:\n"
        "- Data coverage and gaps:\n"
        "- Open audit findings on the changed path:\n"
        "\n"
        "## Results\n"
        "\n"
        "| Metric | Run | Baseline | Difference |\n"
        "|---|---|---|---|\n"
        "| Return | | | |\n"
        "| Max drawdown | | | |\n"
        "| Trades | | | |\n"
        "| Profit factor | | | |\n"
        "| Win rate | | | |\n"
        "| Exposure | | | |\n"
        "| Cost drag | | | |\n"
        "\n"
        "## Observations\n"
        "\n"
        "## Limitations\n"
        "\n"
        "## Verdict\n"
        "\n"
        "Verdict: pending\n"
    )


def decision_text(manifest: dict[str, Any]) -> str:
    frozen = ", ".join(manifest["frozen"]) if manifest["frozen"] else "(none)"
    return (
        f"# Decision: {manifest['run_id']}\n"
        "\n"
        f"- Tier: {manifest['tier']}\n"
        f"- Changed: {manifest['changed']}\n"
        f"- Frozen: {frozen}\n"
        f"- Partition: {manifest['partition']}\n"
        f"- Baseline: {baseline_label(manifest)}\n"
        f"- Holdout access: {manifest['holdout_access']}\n"
        f"- Hypothesis: {manifest['hypothesis']}\n"
        "- Decided by: <role>\n"
        "- Date: <YYYY-MM-DD> (UTC)\n"
        "\n"
        "## Decision\n"
        "\n"
        "Verdict: pending\n"
        "\n"
        "## Rationale\n"
        "\n"
        "## Next hypothesis\n"
    )


def write_text(path: Path, text: str) -> None:
    try:
        path.write_bytes(text.encode("utf-8"))
    except OSError as error:
        raise ToolError(f"cannot write {path}: {error}") from None


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    manifest_path = Path(args.manifest).expanduser()
    if not manifest_path.is_file():
        print(f"error: manifest not found: {manifest_path}", file=sys.stderr)
        return 1
    manifest_path = manifest_path.resolve()

    try:
        manifest = load_manifest(manifest_path)
        root = output_root(manifest)
        ensure_empty_root(root)
        shell = resolve_shell(args.qr)
        config_path = resolve_run_config(manifest_path, manifest["run_config"])
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    try:
        root.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(manifest_path, root / "manifest.json")
        prepare_config(config_path, root / "config")
        (root / "artifacts").mkdir(exist_ok=True)
        write_text(root / "review.md", review_text(manifest))
        write_text(root / "decision.md", decision_text(manifest))
    except (OSError, ToolError) as error:
        print(f"error: cannot prepare run root {root}: {error}", file=sys.stderr)
        return 1

    command = [
        str(shell),
        "backtest",
        "--config",
        str(config_path),
        "--out",
        str(root / "artifacts"),
        "--json",
    ]
    try:
        process = run_process(command, REPO_ROOT)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    stdout_json = None
    stdout_json_error = None
    try:
        stdout_json = json.loads(process.stdout)
    except json.JSONDecodeError as error:
        stdout_json_error = (
            f"stdout is not one JSON document: {error.msg} "
            f"(line {error.lineno}, column {error.colno})"
        )

    inventory = file_inventory(root / "artifacts")
    result = {
        "schema": RESULT_SCHEMA,
        "run_id": manifest["run_id"],
        "tier": manifest["tier"],
        "command": command,
        "return_code": process.returncode,
        "ok": process.returncode == 0,
        "stdout": process.stdout,
        "stderr": process.stderr,
        "stdout_json": stdout_json,
        "stdout_json_error": stdout_json_error,
        "artifacts": inventory,
        "failure": (
            None
            if process.returncode == 0
            else f"shell exited with code {process.returncode}"
        ),
    }
    try:
        write_json(root / "results.json", result)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if process.returncode != 0:
        if process.stderr.strip():
            print("shell stderr:", file=sys.stderr)
            print(process.stderr.rstrip(), file=sys.stderr)
        print(
            f"error: shell exited with code {process.returncode}; "
            f"see {root / 'results.json'}",
            file=sys.stderr,
        )
        return 1

    if not is_within(root, INDEX_ROOT):
        print(
            "note: output root is outside data/improvement; "
            "the canonical index does not include it",
            file=sys.stderr,
        )
    try:
        INDEX_ROOT.mkdir(parents=True, exist_ok=True)
        index = run_process(
            [sys.executable, str(IMPROVEMENT_DIR / "index.py"), "--root", str(INDEX_ROOT)],
            REPO_ROOT,
        )
    except ToolError as error:
        print(f"error: run completed but index failed: {error}", file=sys.stderr)
        return 1

    if index.stdout.strip():
        print(index.stdout.rstrip())
    if index.returncode != 0:
        if index.stderr.strip():
            print(index.stderr.rstrip(), file=sys.stderr)
        print("error: run completed but index failed", file=sys.stderr)
        return 1

    print(f"ok {manifest['run_id']} -> {root}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
