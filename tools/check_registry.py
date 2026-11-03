#!/usr/bin/env python3
"""Validate the indicator and parameter registry and its generated table.

The registry is documentation metadata, not a runtime config source. This
checker fails when the JSON is malformed, a code is duplicated, an event
references an unknown source, a default config disagrees with the registry, or
the generated Markdown is stale.

Usage:
    python3 tools/check_registry.py
    python3 tools/check_registry.py --write   # regenerate docs/registry/indicators.md
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "docs/registry/indicators.json"
MARKDOWN = ROOT / "docs/registry/indicators.md"
SCHEMA = ROOT / "docs/registry/schema.json"

ROOT_KEYS = {"schema", "version", "indicators", "events", "parameters", "codes"}
INDICATOR_KEYS = {
    "id", "code", "name", "taxonomy", "role", "claim", "formula", "inputs",
    "parameters", "outputs", "availability", "events", "tests", "status",
}
EVENT_KEYS = {
    "id", "event_type", "subtype", "source", "indicator", "role", "claim",
    "parameters", "tests", "status", "sources", "precedence",
}
PARAMETER_KEYS = {
    "id", "path", "type", "default", "min", "max", "enum", "unit",
    "configurable", "affects", "status",
}
EVENT_REQUIRED = EVENT_KEYS - {"sources", "precedence"}
PARAMETER_REQUIRED = PARAMETER_KEYS - {"min", "max", "enum"}
CODE_KEYS = {
    "event_types", "level_subtypes", "divergence_subtypes", "signal_subtypes",
    "sources", "roles", "scales", "target_bases", "lifecycles", "stages",
    "unavailable_reasons",
}
CODE_ORDER = [
    "event_types", "level_subtypes", "divergence_subtypes", "signal_subtypes",
    "sources", "roles", "scales", "target_bases", "lifecycles", "stages",
    "unavailable_reasons",
]
ROLES = {"signal", "context", "diagnostic"}
STATUSES = {"active", "planned", "retired", "experimental"}
TAXONOMIES = {"trend", "momentum", "structure", "progression", "volatility"}
AFFECTS = {"model", "strategy", "policy", "execution", "portfolio", "analytics"}
PARAMETER_TYPES = {
    "integer", "number", "boolean", "string", "array<integer>", "array<number>",
    "array<string>", "array<array<integer>>", "object",
}
UNITS = {"bars", "fraction", "atr", "seconds", "bps", "currency", "period", "none"}
TEST_KINDS = {
    "golden", "causality", "determinism", "edge_cases", "property", "event",
    "boundary", "benchmark",
}
SOURCE_RANGE = range(1, 16)
SUBTYPE_RANGE = {1: range(1, 4), 2: range(1, 5), 3: range(1, 6)}


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def check(registry: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(registry, dict):
        return ["registry root must be an object"]
    missing = ROOT_KEYS - set(registry)
    extra = set(registry) - ROOT_KEYS
    if missing:
        errors.append(f"root missing keys: {sorted(missing)}")
    if extra:
        errors.append(f"root unknown keys: {sorted(extra)}")
    if registry.get("schema") != "quantrule.registry/v1":
        errors.append("schema must be quantrule.registry/v1")
    if not isinstance(registry.get("version"), str) or not registry["version"]:
        errors.append("version must be a non-empty string")

    indicators = registry.get("indicators")
    events = registry.get("events")
    parameters = registry.get("parameters")
    codes = registry.get("codes")
    if not isinstance(indicators, list) or not isinstance(events, list) or not isinstance(parameters, list):
        errors.append("indicators, events, and parameters must be arrays")
        return errors
    if not isinstance(codes, dict):
        errors.append("codes must be an object")
        return errors

    errors.extend(check_codes(codes))

    indicator_ids: set[str] = set()
    indicator_codes: set[int] = set()
    for index, indicator in enumerate(indicators):
        label = f"indicators[{index}]"
        if not isinstance(indicator, dict):
            errors.append(f"{label} must be an object")
            continue
        errors.extend(required_and_unknown(label, indicator, INDICATOR_KEYS, INDICATOR_KEYS))
        ident = indicator.get("id")
        if not isinstance(ident, str) or not ident:
            errors.append(f"{label}.id must be a non-empty string")
        elif ident in indicator_ids:
            errors.append(f"{label}.id duplicate: {ident}")
        else:
            indicator_ids.add(ident)
        code = indicator.get("code")
        if not isinstance(code, int) or code < 1:
            errors.append(f"{label}.code must be a positive integer")
        elif code in indicator_codes:
            errors.append(f"{label}.code duplicate: {code}")
        else:
            indicator_codes.add(code)
        if indicator.get("taxonomy") not in TAXONOMIES:
            errors.append(f"{label}.taxonomy is invalid")
        if indicator.get("role") not in ROLES:
            errors.append(f"{label}.role is invalid")
        if indicator.get("status") not in STATUSES:
            errors.append(f"{label}.status is invalid")
        for field in ("name", "claim", "formula", "availability"):
            if not isinstance(indicator.get(field), str) or not indicator[field]:
                errors.append(f"{label}.{field} must be a non-empty string")
        for field in ("inputs", "parameters", "outputs", "events", "tests"):
            values = indicator.get(field)
            if not isinstance(values, list) or not values:
                if field not in ("events",):
                    errors.append(f"{label}.{field} must be a non-empty array")
            if not isinstance(values, list):
                continue
            for value in values:
                if not isinstance(value, str) or not value:
                    errors.append(f"{label}.{field} contains an invalid value")
        errors.extend(check_tests(f"{label}.tests", indicator.get("tests")))
        for event_id in indicator.get("events", []) if isinstance(indicator.get("events"), list) else []:
            if not any(isinstance(event, dict) and event.get("id") == event_id for event in events):
                errors.append(f"{label}.events references unknown event {event_id}")

    parameter_ids: set[str] = set()
    for index, parameter in enumerate(parameters):
        label = f"parameters[{index}]"
        if not isinstance(parameter, dict):
            errors.append(f"{label} must be an object")
            continue
        errors.extend(required_and_unknown(label, parameter, PARAMETER_REQUIRED, PARAMETER_KEYS))
        ident = parameter.get("id")
        if not isinstance(ident, str) or not ident:
            errors.append(f"{label}.id must be a non-empty string")
        elif ident in parameter_ids:
            errors.append(f"{label}.id duplicate: {ident}")
        else:
            parameter_ids.add(ident)
        if not isinstance(parameter.get("path"), str) or not parameter["path"]:
            errors.append(f"{label}.path must be a non-empty string")
        if parameter.get("type") not in PARAMETER_TYPES:
            errors.append(f"{label}.type is invalid")
        if "default" not in parameter:
            errors.append(f"{label}.default is required")
        if not isinstance(parameter.get("configurable"), bool):
            errors.append(f"{label}.configurable must be a boolean")
        if parameter.get("unit") not in UNITS:
            errors.append(f"{label}.unit is invalid")
        if parameter.get("status") not in STATUSES:
            errors.append(f"{label}.status is invalid")
        affects = parameter.get("affects")
        if not isinstance(affects, list) or not affects or any(value not in AFFECTS for value in affects):
            errors.append(f"{label}.affects is invalid")

    for index, event in enumerate(events):
        label = f"events[{index}]"
        if not isinstance(event, dict):
            errors.append(f"{label} must be an object")
            continue
        errors.extend(required_and_unknown(label, event, EVENT_REQUIRED, EVENT_KEYS))
        ident = event.get("id")
        if not isinstance(ident, str) or not ident:
            errors.append(f"{label}.id must be a non-empty string")
        event_type = event.get("event_type")
        if event_type not in SUBTYPE_RANGE:
            errors.append(f"{label}.event_type is invalid")
        elif event.get("subtype") not in SUBTYPE_RANGE[event_type]:
            errors.append(f"{label}.subtype is outside the type's range")
        if event.get("indicator") not in indicator_ids:
            errors.append(f"{label}.indicator is unknown")
        if event.get("role") not in ROLES:
            errors.append(f"{label}.role is invalid")
        if event.get("status") not in STATUSES:
            errors.append(f"{label}.status is invalid")
        if not isinstance(event.get("claim"), str) or not event["claim"]:
            errors.append(f"{label}.claim must be a non-empty string")
        sources = event.get("sources", [event.get("source")])
        if not isinstance(sources, list) or not sources:
            errors.append(f"{label}.sources must be a non-empty array")
        else:
            for source in sources:
                if source not in SOURCE_RANGE or str(source) not in codes.get("sources", {}):
                    errors.append(f"{label} references unknown source {source}")
        for parameter_id in event.get("parameters", []):
            if parameter_id not in parameter_ids:
                errors.append(f"{label}.parameters references unknown parameter {parameter_id}")
        errors.extend(check_tests(f"{label}.tests", event.get("tests")))
        if event.get("role") in ("signal", "context") and not any(
            test in TEST_KINDS for test in event.get("tests", [])
        ):
            errors.append(f"{label} needs at least one test kind")

    errors.extend(check_defaults(registry))
    errors.extend(check_markdown(registry))
    return errors


def required_and_unknown(label: str, value: dict[str, Any], required: set[str], allowed: set[str]) -> list[str]:
    errors: list[str] = []
    missing = required - set(value)
    extra = set(value) - allowed
    if missing:
        errors.append(f"{label} missing keys: {sorted(missing)}")
    if extra:
        errors.append(f"{label} unknown keys: {sorted(extra)}")
    return errors


def check_tests(label: str, tests: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(tests, list) or not tests:
        errors.append(f"{label} must be a non-empty array")
        return errors
    for test in tests:
        if test not in TEST_KINDS:
            errors.append(f"{label} has unknown test kind {test}")
    return errors


def check_codes(codes: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    missing = CODE_KEYS - set(codes)
    extra = set(codes) - CODE_KEYS
    if missing:
        errors.append(f"codes missing keys: {sorted(missing)}")
    if extra:
        errors.append(f"codes unknown keys: {sorted(extra)}")
    for key in CODE_KEYS:
        value = codes.get(key)
        if not isinstance(value, dict) or not value:
            errors.append(f"codes.{key} must be a non-empty object")
            continue
        for code, label in value.items():
            if not code.isdigit():
                errors.append(f"codes.{key} has a non-numeric key {code}")
            if not isinstance(label, str) or not label:
                errors.append(f"codes.{key}.{code} must be a non-empty string")
    return errors


def check_defaults(registry: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    parameters = {entry["id"]: entry for entry in registry.get("parameters", []) if isinstance(entry, dict) and "id" in entry}
    configs = {
        "model": ROOT / "config/defaults/model.json",
        "policy": ROOT / "config/defaults/policy.json",
        "portfolio": ROOT / "config/defaults/portfolio.json",
        "execution": ROOT / "config/examples/btcusd-2024/execution.json",
        "analytics": ROOT / "config/examples/btcusd-2024/analytics.json",
    }
    loaded = {name: load(path) for name, path in configs.items()}
    for ident, parameter in parameters.items():
        if not parameter.get("configurable"):
            continue
        path = parameter.get("path", "")
        if path in ("fixed",) or path.startswith(("weights", "entry.", "risk.", "benchmark.")):
            continue
        config_name = identify_config(ident)
        if config_name is None:
            errors.append(f"{ident} is configurable but has no default config owner")
            continue
        value = nested(loaded[config_name], path)
        if value is None:
            errors.append(f"{ident}: {config_name} default is missing at {path}")
        elif value != parameter.get("default"):
            errors.append(
                f"{ident}: {config_name} default {value!r} != registry {parameter.get('default')!r}"
            )
    return errors


def identify_config(ident: str) -> str | None:
    if ident.startswith("model."):
        return "model"
    if ident.startswith("policy."):
        return "policy"
    if ident.startswith("portfolio."):
        return "portfolio"
    if ident.startswith("execution."):
        return "execution"
    if ident.startswith("analytics."):
        return "analytics"
    return None


def nested(value: Any, path: str) -> Any:
    current = value
    for part in path.split("."):
        if not isinstance(current, dict) or part not in current:
            return None
        current = current[part]
    return current


def render_markdown(registry: dict[str, Any]) -> str:
    lines: list[str] = []
    lines.append("# Indicator and Parameter Registry")
    lines.append("")
    lines.append(f"- Version: `{registry['version']}`")
    lines.append("- Source: [`indicators.json`](indicators.json)")
    lines.append("- Generated by `python3 tools/check_registry.py --write`; do not edit by hand.")
    lines.append("")
    lines.append("## Indicators")
    lines.append("")
    lines.append("| ID | Code | Taxonomy | Role | Status | Claim |")
    lines.append("|---|---:|---|---|---|---|")
    for indicator in registry["indicators"]:
        lines.append(
            f"| `{indicator['id']}` | {indicator['code']} | {indicator['taxonomy']} | "
            f"{indicator['role']} | {indicator['status']} | {cell(indicator['claim'])} |"
        )
    lines.append("")
    lines.append("## Events")
    lines.append("")
    lines.append("| ID | Type | Subtype | Sources | Indicator | Role | Status | Claim |")
    lines.append("|---|---:|---:|---|---|---|---|---|")
    for event in registry["events"]:
        sources = event.get("sources", [event["source"]])
        lines.append(
            f"| `{event['id']}` | {event['event_type']} | {event['subtype']} | "
            f"{'/'.join(str(source) for source in sources)} | {event['indicator']} | "
            f"{event['role']} | {event['status']} | {cell(event['claim'])} |"
        )
    lines.append("")
    lines.append("## Parameters")
    lines.append("")
    lines.append("| ID | Path | Default | Unit | Configurable | Affects | Status |")
    lines.append("|---|---|---|---|---|---|---|")
    for parameter in registry["parameters"]:
        default = json.dumps(parameter["default"], ensure_ascii=False)
        lines.append(
            f"| `{parameter['id']}` | `{parameter['path']}` | `{default}` | "
            f"{parameter['unit']} | {str(parameter['configurable']).lower()} | "
            f"{', '.join(parameter['affects'])} | {parameter['status']} |"
        )
    lines.append("")
    lines.append("## Codes")
    lines.append("")
    for key in CODE_ORDER:
        value = registry["codes"].get(key, {})
        entries = ", ".join(f"{code} = {label}" for code, label in value.items())
        lines.append(f"- **{key}**: {entries}")
    lines.append("")
    return "\n".join(lines)


def cell(value: str) -> str:
    return " ".join(value.split()).replace("|", "\\|")


def check_markdown(registry: dict[str, Any]) -> list[str]:
    current = MARKDOWN.read_text(encoding="utf-8") if MARKDOWN.is_file() else ""
    expected = render_markdown(registry)
    if current != expected:
        return [
            f"{MARKDOWN.relative_to(ROOT)} is stale; run "
            "`python3 tools/check_registry.py --write`"
        ]
    return []


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="regenerate indicators.md")
    args = parser.parse_args(argv)
    registry = load(REGISTRY)
    errors = check(registry)
    if args.write:
        MARKDOWN.write_text(render_markdown(registry), encoding="utf-8")
        errors = [error for error in errors if "is stale" not in error]
        if errors:
            for error in errors:
                print(f"error: {error}", file=sys.stderr)
            return 1
        print(f"wrote {MARKDOWN.relative_to(ROOT)}")
        return 0
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"registry ok: {len(registry['indicators'])} indicators, {len(registry['events'])} events, {len(registry['parameters'])} parameters")
    return 0


if __name__ == "__main__":
    sys.exit(main())
