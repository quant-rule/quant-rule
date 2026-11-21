//! Compare configurable parameter defaults with the committed configs.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value};

use crate::report::Diagnostic;

use super::markdown::render_default;
use super::{finding, load};

pub(super) fn check_defaults(
    root: &Path,
    registry: &Path,
    parameters: &[Value],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut by_id: BTreeMap<&str, &Map<String, Value>> = BTreeMap::new();
    for parameter in parameters {
        if let (Some(ident), Some(parameter)) = (
            parameter.get("id").and_then(Value::as_str),
            parameter.as_object(),
        ) {
            by_id.insert(ident, parameter);
        }
    }
    let configs = [
        ("model", "config/defaults/model.json"),
        ("portfolio", "config/defaults/portfolio.json"),
        ("execution", "config/examples/btcusd-2024/execution.json"),
    ];
    let mut loaded = BTreeMap::new();
    for (name, relative) in configs {
        let path = root.join(relative);
        match load(&path, root) {
            Ok(value) => {
                loaded.insert(name, value);
            }
            Err(()) => diagnostics.push(finding(&path, root, format!("cannot read {relative}"))),
        }
    }
    for (ident, parameter) in by_id {
        if parameter.get("configurable").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let path = parameter.get("path").and_then(Value::as_str).unwrap_or("");
        if path == "fixed" {
            diagnostics.push(finding(
                registry,
                root,
                format!("{ident}: a fixed parameter cannot be configurable"),
            ));
            continue;
        }
        let Some(config_name) = identify_config(ident) else {
            diagnostics.push(finding(
                registry,
                root,
                format!("{ident} is configurable but has no default config owner"),
            ));
            continue;
        };
        let Some(config) = loaded.get(config_name) else {
            continue;
        };
        let default = parameter.get("default").cloned().unwrap_or(Value::Null);
        match nested(config, path) {
            None => diagnostics.push(finding(
                registry,
                root,
                format!("{ident}: {config_name} default is missing at {path}"),
            )),
            Some(value) if !json_equal(value, &default) => diagnostics.push(finding(
                registry,
                root,
                format!(
                    "{ident}: {config_name} default {} != registry {}",
                    render_default(value),
                    render_default(&default)
                ),
            )),
            Some(_) => {}
        }
    }
}

fn identify_config(ident: &str) -> Option<&'static str> {
    if ident.starts_with("model.") {
        Some("model")
    } else if ident.starts_with("policy.") {
        Some("policy")
    } else if ident.starts_with("portfolio.") {
        Some("portfolio")
    } else if ident.starts_with("execution.") {
        Some("execution")
    } else {
        None
    }
}

fn nested<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}

fn json_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right.get(key).is_some_and(|other| json_equal(value, other))
                })
        }
        _ => left == right,
    }
}
