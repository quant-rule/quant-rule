#!/usr/bin/env python3
"""Validate and execute one improvement run on the current S3 command surface.

The manifest is the contract. This command refuses a non-empty output root,
copies the manifest and its configuration for provenance, runs ``report model``
or ``backtest run`` (and, for the strategy and portfolio tiers, ``qualify`` and
``analytics evaluate``), records every command and its observed streams, and
refreshes the local index. A non-zero exit is a failure, not a partially
successful run.

The historical ``run.py`` targets the retired ``backtest`` command and is kept
only for reproducibility. New runs use this adapter.
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
    RESULT_SCHEMA_V2,
    ToolError,
    file_inventory,
    load_manifest,
    repo_relative,
    resolve_shell,
    run_process,
    write_json,
)
import run as historical

DEFAULT_SHELL = "target/debug/quantrule"
INDEX_ROOT = REPO_ROOT / "data" / "improvement"


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="run_current.py",
        description=(
            "Validate a v2 manifest, create its run folder, execute the current "
            "S3 commands, write results.json, and refresh the store index. The "
            "command refuses a non-empty run folder and never treats a non-zero "
            "exit as success."
        ),
        epilog=(
            "Example: python3 tools/improvement/run_current.py "
            "data/improvement/2026-11-07/MODEL-2026-11-07-01/manifest.json "
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


def resolve_path(manifest_path: Path, value: str) -> Path:
    path = Path(value).expanduser()
    candidates = (
        [path] if path.is_absolute() else [manifest_path.parent / path, REPO_ROOT / path]
    )
    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()
    tried = ", ".join(str(candidate) for candidate in candidates)
    raise ToolError(f"path not found: {value!r} (tried {tried})")


def copy_config(source: Path, destination: Path, prefix: str = "") -> str:
    name = source.name
    if (destination / name).exists():
        name = f"{prefix}{name}"
    shutil.copyfile(source, destination / name)
    return name


def prepare_config(
    manifest_path: Path,
    manifest: dict[str, Any],
    config_dir: Path,
) -> dict[str, str]:
    config_dir.mkdir(parents=True, exist_ok=True)
    run_config = resolve_path(manifest_path, manifest["run_config"])
    files: list[dict[str, str]] = []
    for source in historical.sibling_configs(run_config):
        name = copy_config(source, config_dir, "run-")
        files.append({"source": repo_relative(source), "dest": name})
    strategy_name = None
    if manifest.get("strategy_config"):
        source = resolve_path(manifest_path, manifest["strategy_config"])
        strategy_name = copy_config(source, config_dir, "strategy-")
        files.append({"source": repo_relative(source), "dest": strategy_name})
    criteria_name = None
    if manifest.get("qualification_criteria"):
        source = resolve_path(manifest_path, manifest["qualification_criteria"])
        criteria_name = copy_config(source, config_dir, "criteria-")
        files.append({"source": repo_relative(source), "dest": criteria_name})
    write_json(
        config_dir / "sources.json",
        {
            "schema": CONFIG_SCHEMA,
            "run_config": repo_relative(run_config),
            "strategy_config": repo_relative(
                resolve_path(manifest_path, manifest["strategy_config"])
            )
            if manifest.get("strategy_config")
            else None,
            "qualification_criteria": repo_relative(
                resolve_path(manifest_path, manifest["qualification_criteria"])
            )
            if manifest.get("qualification_criteria")
            else None,
            "files": files,
        },
    )
    return {
        "run_config": str(run_config),
        "strategy_config": str(resolve_path(manifest_path, manifest["strategy_config"]))
        if manifest.get("strategy_config")
        else "",
        "qualification_criteria": str(
            resolve_path(manifest_path, manifest["qualification_criteria"])
        )
        if manifest.get("qualification_criteria")
        else "",
    }


def commands_for(
    manifest: dict[str, Any],
    qr: Path,
    root: Path,
    paths: dict[str, str],
) -> list[list[str]]:
    artifacts = root / "artifacts"
    if manifest["tier"] == "model":
        return [
            [
                str(qr),
                "report",
                "model",
                "--config",
                paths["run_config"],
                "--out",
                str(artifacts),
                "--json",
            ]
        ]

    commands = [
        [
            str(qr),
            "backtest",
            "run",
            "--config",
            paths["run_config"],
            "--strategy",
            paths["strategy_config"],
            "--out",
            str(artifacts),
            "--json",
        ]
    ]
    if paths["qualification_criteria"]:
        commands.append(
            [
                str(qr),
                "qualify",
                "--evidence",
                str(artifacts / "backtest"),
                "--criteria",
                paths["qualification_criteria"],
                "--out",
                str(artifacts),
                "--json",
            ]
        )
    if manifest.get("analytics", True):
        commands.append(
            [
                str(qr),
                "analytics",
                "evaluate",
                "--evidence",
                str(artifacts / "backtest"),
                "--out",
                str(artifacts),
                "--json",
            ]
        )
    return commands


def parse_stdout(stdout: str) -> tuple[Any, str | None]:
    try:
        return json.loads(stdout), None
    except json.JSONDecodeError as error:
        return None, (
            f"stdout is not one JSON document: {error.msg} "
            f"(line {error.lineno}, column {error.colno})"
        )


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    manifest_path = Path(args.manifest).expanduser()
    if not manifest_path.is_file():
        print(f"error: manifest not found: {manifest_path}", file=sys.stderr)
        return 1
    manifest_path = manifest_path.resolve()

    try:
        manifest = load_manifest(manifest_path)
        root = historical.output_root(manifest)
        historical.ensure_empty_root(root)
        shell = resolve_shell(args.qr)
        paths = prepare_config(manifest_path, manifest, root / "config")
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    try:
        root.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(manifest_path, root / "manifest.json")
        (root / "artifacts").mkdir(exist_ok=True)
        historical.write_text(root / "review.md", historical.review_text(manifest))
        historical.write_text(root / "decision.md", historical.decision_text(manifest))
    except (OSError, ToolError) as error:
        print(f"error: cannot prepare run root {root}: {error}", file=sys.stderr)
        return 1

    records: list[dict[str, Any]] = []
    failed = False
    for command in commands_for(manifest, shell, root, paths):
        try:
            process = run_process(command, REPO_ROOT)
        except ToolError as error:
            print(f"error: {error}", file=sys.stderr)
            failed = True
            records.append(
                {
                    "command": command,
                    "return_code": None,
                    "ok": False,
                    "stdout": "",
                    "stderr": str(error),
                    "stdout_json": None,
                    "stdout_json_error": None,
                }
            )
            break
        stdout_json, stdout_json_error = parse_stdout(process.stdout)
        records.append(
            {
                "command": command,
                "return_code": process.returncode,
                "ok": process.returncode == 0,
                "stdout": process.stdout,
                "stderr": process.stderr,
                "stdout_json": stdout_json,
                "stdout_json_error": stdout_json_error,
            }
        )
        if process.returncode != 0:
            failed = True
            break

    result = {
        "schema": RESULT_SCHEMA_V2,
        "run_id": manifest["run_id"],
        "tier": manifest["tier"],
        "commands": records,
        "ok": not failed,
        "artifacts": file_inventory(root / "artifacts"),
        "failure": None if not failed else "a command exited non-zero",
    }
    try:
        write_json(root / "results.json", result)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if failed:
        for record in records:
            if not record["ok"] and record["stderr"].strip():
                print("shell stderr:", file=sys.stderr)
                print(record["stderr"].rstrip(), file=sys.stderr)
        print(
            f"error: run failed; see {root / 'results.json'}",
            file=sys.stderr,
        )
        return 1

    if not historical.is_within(root, INDEX_ROOT):
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
