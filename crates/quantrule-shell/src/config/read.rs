//! The strict JSON reader shared by every config file.
//!
//! The reader does not deserialize into a struct. It walks a
//! [`serde_json::Value`] so every rejection can name the exact JSON path, and
//! it rejects what the contract rejects: non-objects at the top level, unknown
//! fields, missing fields, wrong types, and non-finite numbers. A number must
//! be a JSON number, never a numeric string; a boolean must be a JSON boolean,
//! never `"true"`.

use serde_json::{Map, Value};

use super::error::ConfigError;

/// The JSON path of the document root.
pub(crate) const ROOT: &str = "$";

/// The path of a named child.
pub(crate) fn child(path: &str, name: &str) -> String {
    format!("{path}.{name}")
}

/// The path of an array element.
pub(crate) fn index(path: &str, position: usize) -> String {
    format!("{path}[{position}]")
}

/// How a value is shown in an error message.
pub(crate) fn observed(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

/// Requires a JSON object.
pub(crate) fn object<'a>(
    file: &str,
    path: &str,
    value: &'a Value,
) -> Result<&'a Map<String, Value>, ConfigError> {
    value
        .as_object()
        .ok_or_else(|| ConfigError::wrong_type(file, path, "an object", &observed(value)))
}

/// Requires a field, or reports it as missing at `path.name`.
pub(crate) fn field<'a>(
    file: &str,
    path: &str,
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a Value, ConfigError> {
    object
        .get(name)
        .ok_or_else(|| ConfigError::missing_field(file, &child(path, name)))
}

/// Rejects any field not in `allowed`.
///
/// `serde_json`'s map is ordered, so the first unknown field is deterministic.
pub(crate) fn check_fields(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), ConfigError> {
    for name in object.keys() {
        if !allowed.contains(&name.as_str()) {
            return Err(ConfigError::unknown_field(
                file,
                &child(path, name),
                name,
            ));
        }
    }
    Ok(())
}

/// Requires the contract's schema string.
pub(crate) fn schema(
    file: &str,
    object: &Map<String, Value>,
    expected: &str,
) -> Result<(), ConfigError> {
    let value = field(file, ROOT, object, "schema")?;
    let found = string(file, &child(ROOT, "schema"), value)?;
    if found != expected {
        return Err(ConfigError::schema(
            file,
            &child(ROOT, "schema"),
            expected,
            found,
        ));
    }
    Ok(())
}

/// Requires a JSON string.
pub(crate) fn string<'a>(
    file: &str,
    path: &str,
    value: &'a Value,
) -> Result<&'a str, ConfigError> {
    value
        .as_str()
        .ok_or_else(|| ConfigError::wrong_type(file, path, "a string", &observed(value)))
}

/// Requires a non-empty JSON string.
pub(crate) fn non_empty_string<'a>(
    file: &str,
    path: &str,
    value: &'a Value,
) -> Result<&'a str, ConfigError> {
    let text = string(file, path, value)?;
    if text.is_empty() {
        return Err(ConfigError::range(
            file,
            path,
            "a non-empty string",
            text,
        ));
    }
    Ok(text)
}

/// Requires a JSON boolean.
pub(crate) fn boolean(file: &str, path: &str, value: &Value) -> Result<bool, ConfigError> {
    value
        .as_bool()
        .ok_or_else(|| ConfigError::wrong_type(file, path, "a boolean", &observed(value)))
}

/// Requires a finite JSON number.
pub(crate) fn number(file: &str, path: &str, value: &Value) -> Result<f64, ConfigError> {
    let number = value
        .as_f64()
        .ok_or_else(|| ConfigError::wrong_type(file, path, "a number", &observed(value)))?;
    if !number.is_finite() {
        return Err(ConfigError::range(
            file,
            path,
            "a finite number",
            &observed(value),
        ));
    }
    Ok(number)
}

/// Requires a non-negative JSON integer.
pub(crate) fn u64(file: &str, path: &str, value: &Value) -> Result<u64, ConfigError> {
    value
        .as_u64()
        .ok_or_else(|| ConfigError::wrong_type(file, path, "an integer", &observed(value)))
}

/// Requires a JSON array.
pub(crate) fn array<'a>(
    file: &str,
    path: &str,
    value: &'a Value,
) -> Result<&'a [Value], ConfigError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| ConfigError::wrong_type(file, path, "an array", &observed(value)))
}
