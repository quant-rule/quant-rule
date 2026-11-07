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
    parser.add_argument(
        "--model",
        default=None,
        help="cell model-config JSON; copied into each prepared run config",
    )
    parser.add_argument("--qr", default=None, help="quantrule binary")
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument("--evaluator", default="tools/improvement/model_prediction.py")
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--family", default="M2", help="model family passed to the evaluator")
    parser.add_argument(
        "--timeframe",
        default=None,
        help="timeframe seconds, or auto to read selection.json",
    )
    parser.add_argument("--selection", default=None, help="selection.json for auto")
    parser.add_argument("--state-file", default=None, help="state-gate JSON")
    parser.add_argument(
        "--condition-file",
        default=None,
        help="pre-registered sequence/confluence/column condition JSON",
    )
    parser.add_argument(
        "--path-menu",
        default=None,
        help="pre-registered exit menu JSON for the descriptive path report",
    )
    parser.add_argument(
        "--condition-evaluator",
        default="tools/improvement/model_condition.py",
    )
    parser.add_argument(
        "--path-evaluator",
        default="tools/improvement/model_path.py",
    )
    parser.add_argument(
        "--instruments",
        default=None,
        help="comma-separated instrument list; defaults to coverage.json or the M2 set",
    )
    parser.add_argument(
        "--count-only",
        action="store_true",
        help="count state-matched predictions and clusters without evaluating outcomes",
    )
    parser.add_argument("--horizon", type=int, default=20)
    parser.add_argument("--target-atr", type=float, default=2.0)
    parser.add_argument("--stop-atr", type=float, default=1.0)
    parser.add_argument("--cost-bps", type=float, default=6.0)
    parser.add_argument("--primary-cells", type=int, default=5)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--timeout", type=int, default=3_600)
    return parser.parse_args(argv)


def write_json(path: Path, document: Any) -> None:
    path.write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def resolve_path(base: Path, value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else (base / path).resolve()


def prepare_run_config(
    run_path: Path, model_path: Path | None, out_dir: Path
) -> Path:
    """Copies a run config and its siblings so a cell can use its own model.

    The copied data source path and every sibling reference are made explicit,
    so the prepared config is independent of the shared research run directory.
    The shell still reads the same schemas; only the model config differs.
    """
    run = json.loads(run_path.read_text(encoding="utf-8"))
    base = run_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)
    data_path = resolve_path(base, str(run["data"]))
    data = json.loads(data_path.read_text(encoding="utf-8"))
    source = data.get("source")
    if isinstance(source, dict) and isinstance(source.get("path"), str):
        source["path"] = str(resolve_path(data_path.parent, source["path"]))
    write_json(out_dir / "data.json", data)
    for key in ("execution", "portfolio", "policy"):
        sibling = resolve_path(base, str(run[key]))
        shutil.copy2(sibling, out_dir / f"{key}.json")
    model_source = model_path if model_path is not None else resolve_path(base, str(run["model"]))
    shutil.copy2(model_source, out_dir / "model.json")
    run["data"] = "data.json"
    run["model"] = "model.json"
    run["execution"] = "execution.json"
    run["portfolio"] = "portfolio.json"
    run["policy"] = "policy.json"
    run["output_root"] = str(out_dir.parent)
    write_json(out_dir / "run.json", run)
    return out_dir / "run.json"


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


def cell_instruments(cell_dir: Path, requested: str | None) -> list[str]:
    if requested:
        values = [value.strip() for value in requested.split(",") if value.strip()]
        if not values:
            raise SystemExit("error: --instruments is empty")
        return values
    coverage = cell_dir / "coverage.json"
    if coverage.is_file():
        try:
            document = json.loads(coverage.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise SystemExit(f"error: cannot read {coverage}: {error}") from None
        values = document.get("instruments")
        if isinstance(values, list) and values and all(
            isinstance(value, str) and value for value in values
        ):
            return values
    return list(INSTRUMENTS)


def resolve_timeframe(
    value: str | None, cell_dir: Path, selection: str | None
) -> int | None:
    if value is None:
        return None
    if value == "auto":
        path = (
            Path(selection).expanduser()
            if selection
            else cell_dir.parents[1] / "selection.json"
        )
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise SystemExit(f"error: cannot read selection {path}: {error}") from None
        timeframe = document.get("timeframe_secs")
        if not isinstance(timeframe, int) or timeframe <= 0:
            raise SystemExit(f"error: {path} has no positive timeframe_secs")
        return timeframe
    try:
        timeframe = int(value)
    except ValueError:
        raise SystemExit(f"error: --timeframe must be seconds or auto, got {value}") from None
    if timeframe <= 0:
        raise SystemExit("error: --timeframe must be positive")
    return timeframe


def cell_spec(cell_dir: Path, instrument: str, timeframe: int | None) -> Path:
    name = (
        f"spec-{instrument}.json"
        if timeframe is None
        else f"spec-{instrument}-{timeframe}.json"
    )
    return cell_dir / name


def load_state_gate(path: Path):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "improvement"))
    import model_prediction

    return model_prediction.load_state_gate(path)


def state_matches_row(row: dict[str, str], gate) -> bool:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "improvement"))
    import model_prediction

    return model_prediction.state_matches(row, gate)


def count_bundle(
    bundle: Path,
    timeframe: int,
    gate,
    horizon: int,
) -> dict[str, int]:
    state_by_bar: dict[int, bool] = {}
    with (bundle / "readings.csv").open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            try:
                if int(row["timeframe_secs"]) != timeframe:
                    continue
                bar = int(row["bar"])
            except (KeyError, TypeError, ValueError):
                continue
            state_by_bar[bar] = state_matches_row(row, gate) if gate else True
    bars: list[int] = []
    with (bundle / "prediction.csv").open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            try:
                if int(row["timeframe_secs"]) != timeframe:
                    continue
                bar = int(row["bar"])
                prediction = int(row["prediction"])
            except (KeyError, TypeError, ValueError):
                continue
            if prediction != 0 and state_by_bar.get(bar, False):
                bars.append(bar)
    bars.sort()
    clusters = 0
    previous: int | None = None
    for bar in bars:
        if previous is None or bar - previous >= horizon:
            clusters += 1
            previous = bar
    return {"n": len(bars), "clusters": clusters}


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


def run_script(
    python: str,
    script: Path,
    arguments: list[str],
    root: Path,
    timeout: int,
    label: str,
) -> None:
    run_command([python, str(script), *arguments], root, timeout, label)


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


def source_bar_timestamps(bundle: Path) -> dict[int, int]:
    """Maps a yearly bundle's bar index to its timestamp."""
    header, rows = read_csv_rows(bundle / "readings.csv")
    try:
        bar_index = header.index("bar")
        time_index = header.index("timestamp")
    except ValueError as error:
        raise SystemExit(f"error: readings header is missing bar/timestamp: {error}") from None
    mapping: dict[int, int] = {}
    for row in rows:
        mapping[int(row[bar_index])] = int(row[time_index])
    return mapping


def remap_index(
    value: str,
    timeframe: int,
    bar_to_timestamp: dict[int, int],
    bar_by_time: dict[tuple[int, int], int],
) -> int | None:
    try:
        bar = int(value)
    except (TypeError, ValueError):
        return None
    timestamp = bar_to_timestamp.get(bar)
    if timestamp is None:
        return None
    return bar_by_time.get((timeframe, timestamp))


def merge_indexed_artifact(
    sources: list[tuple[int, Path]],
    filename: str,
    bar_by_time: dict[tuple[int, int], int],
    occurred_column: int,
    available_column: int,
) -> tuple[list[str], list[list[str]]] | None:
    """Merges events/levels and remaps their yearly bar indices."""
    header: list[str] | None = None
    rows: list[list[str]] = []
    found = False
    for _, bundle in sources:
        path = bundle / filename
        if not path.is_file():
            continue
        found = True
        source_header, source_rows = read_csv_rows(path)
        if header is None:
            header = source_header
        elif source_header != header:
            raise SystemExit(f"error: {filename} headers differ across years")
        bar_to_timestamp = source_bar_timestamps(bundle)
        for row in source_rows:
            try:
                timeframe = int(row[0])
            except (TypeError, ValueError):
                continue
            occurred = remap_index(
                row[occurred_column], timeframe, bar_to_timestamp, bar_by_time
            )
            available = remap_index(
                row[available_column], timeframe, bar_to_timestamp, bar_by_time
            )
            if occurred is None or available is None:
                continue
            row = list(row)
            row[occurred_column] = str(occurred)
            row[available_column] = str(available)
            rows.append(row)
    if not found or header is None:
        return None
    rows.sort(key=lambda row: (int(row[0]), int(row[available_column]), int(row[1])))
    return header, rows


def merge_plain_artifact(
    sources: list[tuple[int, Path]], filename: str, key_columns: tuple[int, ...]
) -> tuple[list[str], list[list[str]]] | None:
    """Merges a timestamp-keyed artifact (for example panel.csv)."""
    header: list[str] | None = None
    rows: list[list[str]] = []
    found = False
    for _, bundle in sources:
        path = bundle / filename
        if not path.is_file():
            continue
        found = True
        source_header, source_rows = read_csv_rows(path)
        if header is None:
            header = source_header
        elif source_header != header:
            raise SystemExit(f"error: {filename} headers differ across years")
        rows.extend(source_rows)
    if not found or header is None:
        return None
    rows.sort(key=lambda row: tuple(int(row[index]) for index in key_columns))
    return header, rows


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
    artifact_counts: dict[str, int] = {}
    for filename, occurred, available in (
        ("events.csv", 6, 7),
        ("levels.csv", 5, 6),
    ):
        merged_artifact = merge_indexed_artifact(
            sources, filename, bar_by_time, occurred, available
        )
        if merged_artifact is None:
            continue
        artifact_header, artifact_rows = merged_artifact
        write_csv_rows(destination / filename, artifact_header, artifact_rows)
        artifact_counts[filename] = len(artifact_rows)
    panel = merge_plain_artifact(sources, "panel.csv", (0,))
    if panel is not None:
        panel_header, panel_rows = panel
        write_csv_rows(destination / "panel.csv", panel_header, panel_rows)
        artifact_counts["panel.csv"] = len(panel_rows)
    manifest = {
        "schema": MERGE_SCHEMA,
        "years": [year for year, _ in sources],
        "readings_rows": len(readings_rows),
        "prediction_rows": len(prediction_rows),
        "artifacts": artifact_counts,
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
    model_path = None
    if args.model:
        model_path = Path(args.model).expanduser()
        if not model_path.is_absolute():
            model_path = root / model_path
        if not model_path.is_file():
            raise SystemExit(f"error: model config not found: {model_path}")
    elif (cell_dir / "model.json").is_file():
        model_path = cell_dir / "model.json"
    partition, split_start, split_end, years = PARTITIONS[args.phase]
    qr = shell_binary(root, args.qr)
    cell_id = cell_dir.name
    timeframe = resolve_timeframe(args.timeframe, cell_dir, args.selection)
    if args.count_only and timeframe is None:
        raise SystemExit("error: --count-only requires --timeframe")
    instruments = cell_instruments(cell_dir, args.instruments)
    state_path = None
    if args.state_file:
        state_path = Path(args.state_file).expanduser()
        if not state_path.is_absolute():
            state_path = root / state_path
    condition_path = None
    if args.condition_file:
        condition_path = Path(args.condition_file).expanduser()
        if not condition_path.is_absolute():
            condition_path = root / condition_path
        if not condition_path.is_file():
            raise SystemExit(f"error: condition file not found: {condition_path}")
    if state_path is not None and condition_path is not None:
        raise SystemExit("error: --state-file and --condition-file are mutually exclusive")
    path_menu = None
    if args.path_menu:
        path_menu = Path(args.path_menu).expanduser()
        if not path_menu.is_absolute():
            path_menu = root / path_menu
        if not path_menu.is_file():
            raise SystemExit(f"error: path menu not found: {path_menu}")
    condition_evaluator = Path(args.condition_evaluator).expanduser()
    if not condition_evaluator.is_absolute():
        condition_evaluator = root / condition_evaluator
    path_evaluator = Path(args.path_evaluator).expanduser()
    if not path_evaluator.is_absolute():
        path_evaluator = root / path_evaluator
    gate = load_state_gate(state_path) if state_path is not None else None
    bundles: list[tuple[str, Path]] = []
    counts: dict[str, dict[str, int]] = {}
    generated_gate: Path | None = None
    for instrument in instruments:
        spec = cell_spec(cell_dir, instrument, timeframe)
        if not spec.is_file():
            raise SystemExit(f"error: spec not found: {spec}")
        yearly: list[tuple[int, Path]] = []
        for year in years:
            run = runs_dir / instrument / str(year) / "run.json"
            if not run.is_file():
                raise SystemExit(f"error: run config not found: {run}")
            year_out = out / instrument / str(year)
            if model_path is not None:
                run = prepare_run_config(run, model_path, year_out / "config")
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
        if args.count_only and condition_path is None:
            values = [
                count_bundle(bundle, timeframe, gate, args.horizon)
                for _, bundle in yearly
            ]
            counts[instrument] = {
                "n": sum(value["n"] for value in values),
                "clusters": sum(value["clusters"] for value in values),
            }
            continue
        merged = out / instrument / "model"
        merge_bundles(merged, yearly)
        bundle = merged
        if condition_path is not None:
            conditioned = out / instrument / "conditioned"
            run_script(
                args.python,
                condition_evaluator,
                [
                    "--bundle",
                    str(merged),
                    "--condition",
                    str(condition_path),
                    "--registry",
                    str(root / "docs/registry/indicators.json"),
                    "--out",
                    str(conditioned),
                ],
                root,
                args.timeout,
                f"{cell_id} {instrument} condition",
            )
            bundle = conditioned
            generated_gate = conditioned / "state.json"
        if args.count_only:
            count_gate = (
                load_state_gate(generated_gate)
                if generated_gate is not None
                else gate
            )
            counts[instrument] = count_bundle(
                bundle, timeframe, count_gate, args.horizon
            )
            continue
        bundles.append((instrument, bundle))
    if args.count_only:
        pooled = {
            "n": sum(value["n"] for value in counts.values()),
            "clusters": sum(value["clusters"] for value in counts.values()),
        }
        document = {
            "schema": "quantrule.model-cell-count/v1",
            "cell_id": cell_id,
            "phase": args.phase,
            "timeframe_secs": timeframe,
            "state_file": (
                str(generated_gate)
                if generated_gate is not None
                else str(state_path)
                if state_path is not None
                else None
            ),
            "condition_file": str(condition_path) if condition_path is not None else None,
            "model_config_sha256": (
                file_digest(model_path) if model_path is not None else None
            ),
            "instruments": instruments,
            "years": list(years),
            "counts": counts,
            "pooled": pooled,
        }
        (out / "count.json").write_text(
            json.dumps(document, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        print(
            f"ok {cell_id} count timeframe={timeframe} n={pooled['n']} "
            f"clusters={pooled['clusters']} -> {out}"
        )
        return 0
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
        args.family,
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
    if timeframe is not None:
        command.extend(["--timeframe", str(timeframe)])
    if generated_gate is not None:
        command.extend(["--state-file", str(generated_gate)])
    elif state_path is not None:
        command.extend(["--state-file", str(state_path)])
    for instrument, bundle in bundles:
        command.extend(["--bundle", f"{instrument}={bundle}"])
    run_command(command, root, args.timeout, f"{cell_id} evaluation")
    document = json.loads(
        (evaluation / "model_prediction.json").read_text(encoding="utf-8")
    )
    if model_path is not None:
        document["model_config_sha256"] = file_digest(model_path)
        document["model_config_path"] = str(model_path)
    if condition_path is not None:
        document["condition_file"] = str(condition_path)
        document["condition_sha256"] = file_digest(condition_path)
    if path_menu is not None:
        path_out = out / "path"
        path_arguments = [
            "--spec-id",
            cell_id,
            "--family",
            args.family,
            "--partition",
            partition,
            "--cost-bps",
            str(args.cost_bps),
            "--menu",
            str(path_menu),
            "--out",
            str(path_out),
        ]
        if condition_path is not None:
            path_arguments.extend(["--condition-column", "condition"])
        for instrument, bundle in bundles:
            path_arguments.extend(["--bundle", f"{instrument}={bundle}"])
        run_script(
            args.python,
            path_evaluator,
            path_arguments,
            root,
            args.timeout,
            f"{cell_id} path",
        )
        path_document = json.loads(
            (path_out / "path.json").read_text(encoding="utf-8")
        )
        document["path"] = {
            "path": str(path_out / "path.json"),
            "menu": path_document.get("menu"),
            "summaries": path_document.get("summaries"),
        }
    write_root_evaluation(out, document, digest, args.phase, years)
    print(
        f"ok {cell_id} phase={args.phase} instruments={len(bundles)} "
        f"years={len(years)} grade={document.get('grade')} "
        f"verdict={document.get('verdict')} digest={digest[:12]}"
    )
    return 0

if __name__ == "__main__":
    sys.exit(main())
