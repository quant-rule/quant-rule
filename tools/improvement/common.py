#!/usr/bin/env python3
"""Shared helpers for the improvement run tooling.

The module owns the manifest schema contract, deterministic JSON I/O, UTC
parsing, and the path and process helpers the scripts share. It never reaches
the network, reads the clock, or invents a value: every check either accepts the
input or returns an explicit error.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
import shutil
import subprocess
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any, Sequence

REPO_ROOT = Path(__file__).resolve().parents[2]
IMPROVEMENT_DIR = Path(__file__).resolve().parent
PARTITIONS_PATH = REPO_ROOT / "data" / "partitions.json"

MANIFEST_SCHEMA = "quantrule.improvement-run/v1"
RESULT_SCHEMA = "quantrule.improvement-result/v1"
EVIDENCE_SCHEMA = "quantrule.improvement-evidence/v1"
INDEX_SCHEMA = "quantrule.improvement-index/v1"
CONFIG_SCHEMA = "quantrule.improvement-config/v1"

TIERS = ("model", "strategy", "portfolio")
FROZEN_COMPONENTS = TIERS
TIER_RANK = {tier: rank for rank, tier in enumerate(TIERS)}
HOLDOUT_PARTITION = "portfolio_holdout"
PARTITION_TIERS = {
    "model_dev": "model",
    "model_validation": "model",
    "strategy_dev": "strategy",
    "strategy_validation": "strategy",
    "portfolio_dev": "portfolio",
    "portfolio_holdout": "portfolio",
}

RUN_ID_RE = re.compile(r"^(MODEL|STRATEGY|PORTFOLIO)-(\d{4}-\d{2}-\d{2})-(\d{2})$")
RUN_ID_TIER = {"MODEL": "model", "STRATEGY": "strategy", "PORTFOLIO": "portfolio"}

REQUIRED_FIELDS = (
    "run_id",
    "tier",
    "hypothesis",
    "created_utc",
    "changed",
    "frozen",
    "partition",
    "run_config",
    "code_commit",
    "seed",
    "green_criteria",
    "holdout_access",
)


class ToolError(Exception):
    """A user-facing failure that the command reports and turns into exit 1."""


def read_json(path: Path) -> Any:
    """Load a JSON document, reporting the path and decode position on failure."""
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ToolError(f"cannot read {path}: {error}") from None
    try:
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise ToolError(
            f"{path}: invalid JSON: {error.msg} "
            f"(line {error.lineno}, column {error.colno})"
        ) from None


def write_json(path: Path, value: Any) -> None:
    """Write stable pretty JSON with a trailing newline and UTF-8 bytes."""
    try:
        text = json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    except (TypeError, ValueError) as error:
        raise ToolError(f"{path}: cannot encode JSON: {error}") from None
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(text.encode("utf-8"))
    except OSError as error:
        raise ToolError(f"cannot write {path}: {error}") from None


def compact_json(value: Any) -> str:
    """Render a value as sorted, compact JSON for a one-line record."""
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
    )


def sha256_file(path: Path) -> str:
    """Return the SHA-256 digest of a file, read in fixed-size chunks."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1 << 20), b""):
                digest.update(chunk)
    except OSError as error:
        raise ToolError(f"cannot read {path}: {error}") from None
    return digest.hexdigest()


def sha256_bytes(value: bytes) -> str:
    """Return the SHA-256 digest of a byte string."""
    return hashlib.sha256(value).hexdigest()


def file_inventory(root: Path) -> list[dict[str, str]]:
    """List files under ``root`` by relative POSIX path with their digest."""
    if not root.is_dir():
        return []
    files = [path for path in root.rglob("*") if path.is_file()]
    files.sort(key=lambda path: path.relative_to(root).as_posix())
    return [
        {"path": path.relative_to(root).as_posix(), "sha256": sha256_file(path)}
        for path in files
    ]


def is_number(value: Any) -> bool:
    """A finite JSON number, excluding booleans."""
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(value)
    )


def sign(value: float) -> int:
    """Return -1, 0, or 1 for a finite number."""
    if value > 0:
        return 1
    if value < 0:
        return -1
    return 0


def mean(values: Sequence[float]) -> float | None:
    """Return the arithmetic mean in input order, or ``None`` for no values."""
    if not values:
        return None
    return math.fsum(values) / len(values)


def parse_utc_timestamp(value: Any) -> datetime | None:
    """Parse an ISO 8601 timestamp that carries an explicit UTC offset.

    A timestamp without an offset is rejected: the contract stores UTC and does
    not let the host timezone decide what a naive value means.
    """
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None or parsed.utcoffset() != timedelta(0):
        return None
    return parsed


def load_partition_names() -> dict[str, Any]:
    """Load the partition map from ``data/partitions.json``."""
    document = read_json(PARTITIONS_PATH)
    if not isinstance(document, dict):
        raise ToolError(f"{PARTITIONS_PATH}: must be a JSON object")
    partitions = document.get("partitions")
    if not isinstance(partitions, dict) or not partitions:
        raise ToolError(f"{PARTITIONS_PATH}: 'partitions' must be a non-empty object")
    return partitions


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def validate_manifest(manifest: Any) -> list[str]:
    """Return every schema error for an improvement run manifest.

    The checks follow ``quantrule.improvement-run/v1``. They are ordered by
    field so two runs over the same bad manifest print identical errors.
    """
    if not isinstance(manifest, dict):
        return ["manifest: must be a JSON object"]

    errors: list[str] = []

    if "schema" not in manifest:
        errors.append("schema: required")
    elif manifest["schema"] != MANIFEST_SCHEMA:
        errors.append(f"schema: must be {MANIFEST_SCHEMA!r}")

    run_id = manifest.get("run_id")
    run_match = None
    if "run_id" not in manifest:
        errors.append("run_id: required")
    elif not _nonempty_string(run_id):
        errors.append("run_id: must be a non-empty string")
    else:
        run_match = RUN_ID_RE.fullmatch(run_id.strip())
        if run_match is None:
            errors.append(
                "run_id: must match "
                "MODEL-YYYY-MM-DD-NN, STRATEGY-YYYY-MM-DD-NN, or "
                "PORTFOLIO-YYYY-MM-DD-NN"
            )
        else:
            try:
                date.fromisoformat(run_match.group(2))
            except ValueError:
                errors.append(f"run_id: invalid calendar date {run_match.group(2)!r}")
            if run_match.group(3) == "00":
                errors.append("run_id: sequence must start at 01")

    tier = manifest.get("tier")
    if "tier" not in manifest:
        errors.append("tier: required")
    elif tier not in TIERS:
        errors.append("tier: must be one of model, strategy, portfolio")

    hypothesis = manifest.get("hypothesis")
    if "hypothesis" not in manifest:
        errors.append("hypothesis: required")
    elif not _nonempty_string(hypothesis):
        errors.append("hypothesis: must be a non-empty string")

    created_utc = manifest.get("created_utc")
    if "created_utc" not in manifest:
        errors.append("created_utc: required")
    elif parse_utc_timestamp(created_utc) is None:
        errors.append(
            "created_utc: must be an ISO 8601 timestamp with a UTC offset, "
            "for example 2026-11-01T12:00:00Z"
        )

    changed = manifest.get("changed")
    if "changed" not in manifest:
        errors.append("changed: required")
    elif changed not in TIERS:
        errors.append("changed: must be one of model, strategy, portfolio")

    frozen = manifest.get("frozen")
    valid_frozen: list[str] = []
    if "frozen" not in manifest:
        errors.append("frozen: required")
    elif not isinstance(frozen, list):
        errors.append("frozen: must be a list of component names")
    else:
        seen: set[str] = set()
        for index, entry in enumerate(frozen):
            if entry not in FROZEN_COMPONENTS:
                errors.append(
                    f"frozen[{index}]: must be one of "
                    + ", ".join(FROZEN_COMPONENTS)
                )
                continue
            if entry in seen:
                errors.append(f"frozen[{index}]: duplicate component {entry!r}")
                continue
            seen.add(entry)
            valid_frozen.append(entry)
        if changed in valid_frozen:
            errors.append(
                f"changed: {changed!r} also appears in frozen; the changed "
                "component must stay out of the frozen set"
            )
        if tier in TIERS and tier in valid_frozen:
            errors.append(
                f"frozen: the run tier {tier!r} must stay out of the frozen set"
            )

    if tier in TIERS and changed in TIERS and changed != tier:
        errors.append(
            f"changed: {changed!r} does not match tier {tier!r}; a run changes "
            "the tier it is named for"
        )

    if run_match is not None and tier in TIERS:
        prefix_tier = RUN_ID_TIER[run_match.group(1)]
        if prefix_tier != tier:
            errors.append(
                f"run_id: prefix {run_match.group(1)!r} does not match tier {tier!r}"
            )

    partition = manifest.get("partition")
    partitions: dict[str, Any] = {}
    try:
        partitions = load_partition_names()
    except ToolError as error:
        errors.append(str(error))
    if "partition" not in manifest:
        errors.append("partition: required")
    elif not _nonempty_string(partition):
        errors.append("partition: must be a non-empty string")
    elif partitions and partition not in partitions:
        names = ", ".join(sorted(partitions))
        errors.append(f"partition: {partition!r} is not declared; known: {names}")
    elif partitions and tier in TIERS and partition in partitions:
        expected_tier = PARTITION_TIERS.get(partition)
        if expected_tier is not None and expected_tier != tier:
            errors.append(
                f"partition: {partition!r} belongs to the {expected_tier} tier, "
                f"not {tier!r}"
            )

    run_config = manifest.get("run_config")
    if "run_config" not in manifest:
        errors.append("run_config: required")
    elif not _nonempty_string(run_config):
        errors.append("run_config: must be a non-empty path")

    if "output_root" in manifest and manifest["output_root"] is not None:
        if not _nonempty_string(manifest["output_root"]):
            errors.append("output_root: must be a non-empty path when present")

    if "baseline" in manifest and manifest["baseline"] is not None:
        baseline = manifest["baseline"]
        if not isinstance(baseline, (str, dict)):
            errors.append("baseline: must be a run-id string or an object when present")
        elif isinstance(baseline, str) and not baseline.strip():
            errors.append("baseline: must be a non-empty run-id string when present")

    code_commit = manifest.get("code_commit")
    if "code_commit" not in manifest:
        errors.append("code_commit: required")
    elif not _nonempty_string(code_commit):
        errors.append("code_commit: must be a non-empty string")

    seed = manifest.get("seed")
    if "seed" not in manifest:
        errors.append("seed: required")
    elif isinstance(seed, bool) or not isinstance(seed, int) or seed < 0:
        errors.append("seed: must be a non-negative integer")

    green_criteria = manifest.get("green_criteria")
    if "green_criteria" not in manifest:
        errors.append("green_criteria: required")
    elif not isinstance(green_criteria, dict) or not green_criteria:
        errors.append("green_criteria: must be a non-empty object")

    holdout_access = manifest.get("holdout_access")
    if "holdout_access" not in manifest:
        errors.append("holdout_access: required")
    elif holdout_access not in ("none", "final"):
        errors.append("holdout_access: must be 'none' or 'final'")
    elif isinstance(partition, str) and partitions and partition in partitions:
        if partition == HOLDOUT_PARTITION and holdout_access != "final":
            errors.append(
                "holdout_access: portfolio_holdout requires 'final'"
            )
        if partition != HOLDOUT_PARTITION and holdout_access == "final":
            errors.append(
                f"holdout_access: {partition!r} is a development partition and "
                "must use 'none'"
            )

    return errors


def load_manifest(path: Path) -> dict[str, Any]:
    """Load and validate a manifest, raising one error that lists every defect."""
    manifest = read_json(path)
    errors = validate_manifest(manifest)
    if errors:
        detail = "\n".join(f"  {error}" for error in errors)
        raise ToolError(f"{path}: invalid improvement manifest\n{detail}")
    return manifest


def resolve_shell(raw: str) -> Path:
    """Resolve the shell binary without running it.

    A path containing a separator is repository-relative; a bare name is
    searched on PATH first and then under the repository root.
    """
    if not raw:
        raise ToolError("shell binary path is empty")
    candidate = Path(raw).expanduser()
    if candidate.is_absolute():
        candidates = [candidate]
    elif "/" in raw or os.sep in raw:
        candidates = [REPO_ROOT / candidate]
    else:
        found = shutil.which(raw)
        if found is not None:
            resolved = Path(found)
            if resolved.is_file() and os.access(resolved, os.X_OK):
                return resolved.resolve()
        candidates = [REPO_ROOT / candidate]

    for path in candidates:
        if path.is_file() and os.access(path, os.X_OK):
            return path.resolve()
    raise ToolError(
        f"shell binary not found or not executable: {raw} "
        "(build it with `cargo build`, or pass --qr <path>)"
    )


def run_process(command: Sequence[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    """Run a command without a shell and capture UTF-8 stdout and stderr."""
    try:
        return subprocess.run(
            list(command),
            cwd=str(cwd),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
    except OSError as error:
        raise ToolError(f"cannot run {command[0]}: {error}") from None


def repo_relative(path: Path) -> str:
    """Return a repository-relative POSIX path, or the absolute path outside."""
    resolved = path.resolve()
    try:
        return resolved.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return str(resolved)


def is_within(path: Path, parent: Path) -> bool:
    """Whether ``path`` resolves inside ``parent``."""
    try:
        path.resolve().relative_to(parent.resolve())
    except ValueError:
        return False
    return True


def markdown_cell(value: Any) -> str:
    """Flatten a value into one table cell without breaking the table."""
    text = "" if value is None else str(value)
    text = " ".join(text.split())
    return text.replace("|", "\\|")


def portfolio_metrics(report: Any) -> tuple[dict[str, Any], list[str]]:
    """Extract the portfolio summary fields evidence and comparison share.

    The returned ``return`` is the report's ``summary.total_return``. Missing
    fields stay null and produce a note rather than a zero.
    """
    fields = (
        "return",
        "max_drawdown",
        "trades",
        "profit_factor",
        "win_rate",
        "halted",
        "final_equity",
    )
    values: dict[str, Any] = {field: None for field in fields}
    notes: list[str] = []
    if not isinstance(report, dict):
        return values, ["portfolio report is not a JSON object"]
    summary = report.get("summary")
    if not isinstance(summary, dict):
        return values, ["portfolio report has no summary object"]

    sources = {
        "return": "total_return",
        "max_drawdown": "max_drawdown",
        "trades": "trades",
        "profit_factor": "profit_factor",
        "win_rate": "win_rate",
        "halted": "halted",
        "final_equity": "final_equity",
    }
    for field, key in sources.items():
        if key not in summary:
            notes.append(f"summary.{key} is missing")
            continue
        value = summary[key]
        if field == "halted":
            if isinstance(value, bool):
                values[field] = value
            else:
                notes.append(f"summary.{key} is not a boolean")
        elif is_number(value):
            values[field] = value
        else:
            notes.append(f"summary.{key} is not a finite number")
    return values, notes


def model_metrics(report: Any) -> tuple[dict[str, Any], list[str]]:
    """Summarize the model report's versions and per-timeframe coverage."""
    values: dict[str, Any] = {
        "model_version": None,
        "view_contract": None,
        "data_version": None,
        "requested_period": None,
        "ladder_seconds": None,
        "timeframes": [],
    }
    notes: list[str] = []
    if not isinstance(report, dict):
        return values, ["model report is not a JSON object"]

    for field in ("model_version", "view_contract", "data_version"):
        value = report.get(field)
        if isinstance(value, str) and value:
            values[field] = value
        else:
            notes.append(f"{field} is missing")
    if isinstance(report.get("requested_period"), dict):
        values["requested_period"] = report["requested_period"]
    else:
        notes.append("requested_period is missing")

    market = report.get("market_report")
    if not isinstance(market, dict):
        notes.append("market_report is missing")
        return values, notes

    ladder = market.get("ladder")
    if isinstance(ladder, list) and all(is_number(item) for item in ladder):
        values["ladder_seconds"] = ladder
    else:
        notes.append("market_report.ladder is missing or not numeric")

    timeframes = market.get("timeframes")
    if not isinstance(timeframes, list):
        notes.append("market_report.timeframes is missing")
        return values, notes
    rows: list[dict[str, Any]] = []
    for index, timeframe in enumerate(timeframes):
        if not isinstance(timeframe, dict):
            notes.append(f"market_report.timeframes[{index}] is not an object")
            continue
        row: dict[str, Any] = {
            "timeframe_secs": None,
            "bars": None,
            "missing_bars": None,
            "coverage_share": None,
        }
        for field in row:
            value = timeframe.get(field)
            if is_number(value):
                row[field] = value
            else:
                notes.append(f"market_report.timeframes[{index}].{field} is missing")
        rows.append(row)
    values["timeframes"] = rows
    return values, notes
