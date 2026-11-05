#!/usr/bin/env python3
"""Result adapters for the research cell runner.

A cell produces one artifact in its output directory. The adapter turns that
artifact into a small metrics record and a pass/fail verdict. The adapter does
not re-run the gate and does not decide validation.

Registered parsers:
- path_validation: tools/improvement/path_validation.py output
- strategy_evaluation: legacy strategy-matrix output (no current producer)
- model_prediction: tools/improvement/model_prediction.py output
- summary_json: a generic summary.json with metrics and verdict
- none: no parser; the exit code is the verdict
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class ParseResult:
    metrics: dict[str, Any]
    verdict: str | None
    limitations: tuple[str, ...]


def _read(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise FileNotFoundError(f"result artifact not found: {path}") from None
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON in {path}: {error}") from None
    if not isinstance(document, dict):
        raise ValueError(f"{path} must contain an object")
    return document


def _metric(row: dict[str, Any], key: str) -> Any:
    return row.get(key)


def parse_path_validation(out_dir: Path) -> ParseResult:
    document = _read(out_dir / "path_validation.json")
    cells = document.get("cells")
    if not isinstance(cells, list) or not cells:
        raise ValueError("path_validation.json has no cells")
    primary_config = document.get("primary_config")
    selection = document.get("selection")
    row: dict[str, Any] | None = None
    if isinstance(selection, dict):
        for candidate in cells:
            if (
                isinstance(candidate, dict)
                and candidate.get("family") == selection.get("family")
                and candidate.get("timeframe") == selection.get("timeframe")
                and candidate.get("config") == selection.get("config")
            ):
                row = candidate
                break
    if row is None:
        primary = [
            candidate
            for candidate in cells
            if isinstance(candidate, dict) and candidate.get("config") == primary_config
        ]
        pool = primary or [candidate for candidate in cells if isinstance(candidate, dict)]
        if not pool:
            raise ValueError("path_validation.json has no usable row")
        row = sorted(
            pool,
            key=lambda value: (
                str(value.get("timeframe", "")),
                str(value.get("family", "")),
            ),
        )[0]
    metrics = {
        "n": _metric(row, "n"),
        "clusters": _metric(row, "clusters"),
        "hit_rate": _metric(row, "hit_rate"),
        "base_rate": _metric(row, "base_rate"),
        "lift": _metric(row, "lift"),
        "lift_lcb": _metric(row, "lift_lcb"),
        "incremental_lift": _metric(row, "incremental_lift"),
        "incremental_lcb": _metric(row, "incremental_lcb"),
        "random_p": _metric(row, "random_p"),
        "back_lag_lift": _metric(row, "back_lag_lift"),
        "shuffle_p": _metric(row, "shuffle_p"),
        "replication": _metric(row, "replication"),
        "grade": _metric(row, "grade"),
    }
    grade = row.get("grade")
    passed = isinstance(selection, dict) and grade in {"A", "B"}
    limitations = ["Barrier path on the model pack; no execution costs."]
    if not passed:
        limitations.append("No primary cell passed the model gate.")
    return ParseResult(metrics, "pass" if passed else "fail", tuple(limitations))


def parse_strategy_evaluation(out_dir: Path) -> ParseResult:
    files = sorted(path for path in out_dir.glob("strategy_*.json") if path.is_file())
    if not files:
        raise FileNotFoundError(f"no strategy_*.json in {out_dir}")
    document = _read(files[0])
    pooled = document.get("pooled")
    if not isinstance(pooled, dict):
        raise ValueError(f"{files[0]} has no pooled summary")
    passed = document.get("passed") is True
    metrics = {
        "average_instrument_return": pooled.get("average_instrument_return"),
        "worst_instrument_drawdown": pooled.get("worst_instrument_drawdown"),
        "trades": pooled.get("trades"),
        "profit_factor": pooled.get("profit_factor"),
        "positive_instruments": pooled.get("positive_instruments"),
        "criteria": document.get("criteria"),
        "partition": document.get("partition"),
    }
    limitations = [
        "Backtest after costs; validation is a separate one-shot tier.",
        "Review the fold table; a pooled result can hide a halted fold.",
    ]
    return ParseResult(metrics, "pass" if passed else "fail", tuple(limitations))


def parse_model_prediction(out_dir: Path) -> ParseResult:
    document = _read(out_dir / "model_prediction.json")
    metrics_doc = document.get("metrics")
    if not isinstance(metrics_doc, dict):
        raise ValueError("model_prediction.json has no metrics object")
    grade = document.get("grade")
    passed = document.get("verdict") == "pass" and grade in {"A", "B"}
    metrics = {
        "n": metrics_doc.get("n"),
        "clusters": metrics_doc.get("clusters"),
        "hit_rate": metrics_doc.get("hit_rate"),
        "base_rate": metrics_doc.get("base_rate"),
        "lift": metrics_doc.get("lift"),
        "lift_lcb": metrics_doc.get("lift_lcb"),
        "lcb_method": metrics_doc.get("lcb_method"),
        "random_p": metrics_doc.get("random_p"),
        "shuffle_p": metrics_doc.get("shuffle_p"),
        "back_lag_lift": metrics_doc.get("back_lag_lift"),
        "replication": metrics_doc.get("replication"),
        "grade": grade,
        "spec_id": document.get("spec_id"),
        "family": document.get("family"),
        "instrument": document.get("instrument"),
        "prediction_digest": document.get("prediction_digest"),
        "phase": document.get("phase"),
        "partition": document.get("partition"),
        "horizon": document.get("horizon"),
        "cost_bps": document.get("cost_bps"),
        "verdict": document.get("verdict"),
    }
    limitations = document.get("limitations", [])
    if not isinstance(limitations, list):
        limitations = [str(limitations)]
    if not passed:
        limitations = list(limitations) + ["The model prediction did not pass the gate."]
    return ParseResult(
        metrics,
        "pass" if passed else "fail",
        tuple(str(item) for item in limitations),
    )


def parse_summary_json(out_dir: Path) -> ParseResult:
    document = _read(out_dir / "summary.json")
    metrics = document.get("metrics", document)
    if not isinstance(metrics, dict):
        raise ValueError("summary.json metrics must be an object")
    verdict = document.get("verdict")
    if verdict not in {"pass", "fail"}:
        verdict = "pass" if document.get("passed") is True else "fail"
    limitations = document.get("limitations", [])
    if not isinstance(limitations, list):
        limitations = [str(limitations)]
    return ParseResult(metrics, verdict, tuple(str(item) for item in limitations))


def parse_none(out_dir: Path) -> ParseResult:
    return ParseResult({}, None, ())


def parse(parser: str, out_dir: Path) -> ParseResult:
    if parser == "path_validation":
        return parse_path_validation(out_dir)
    if parser == "strategy_evaluation":
        return parse_strategy_evaluation(out_dir)
    if parser == "model_prediction":
        return parse_model_prediction(out_dir)
    if parser == "summary_json":
        return parse_summary_json(out_dir)
    if parser == "none":
        return parse_none(out_dir)
    raise ValueError(f"unknown result parser: {parser}")
