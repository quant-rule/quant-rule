//! Canonical JSON bytes for a content-addressed pack.
//!
//! A pack identity must not depend on object key order, whitespace, or a
//! serializer's map implementation. The writer sorts every object key, emits no
//! insignificant whitespace, keeps the numeric representation `serde_json`
//! stored, writes UTF-8 without a BOM, and appends exactly one trailing newline.
//! It is deliberately independent of `serde_json`'s object representation so a
//! feature change elsewhere cannot silently move every pack ID.

use serde_json::Value;

/// Returns the canonical JSON text of `value`.
///
/// Object keys are sorted lexicographically, arrays keep their order, and the
/// result ends with one newline. The bytes are UTF-8 and carry no BOM.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output.push('\n');
    output
}

fn write_value(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(flag) => output.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => output.push_str(&number.to_string()),
        Value::String(text) => write_string(text, output),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(value, output);
            }
            output.push(']');
        }
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort_unstable();
            output.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_string(key, output);
                output.push(':');
                if let Some(value) = object.get(key) {
                    write_value(value, output);
                }
            }
            output.push('}');
        }
    }
}

fn write_string(text: &str, output: &mut String) {
    output.push('"');
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control <= '\u{1f}' => {
                output.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::canonical_json;

    #[test]
    fn keys_are_sorted_and_whitespace_is_removed() {
        let value = json!({
            "version": "1.0.0",
            "payload": {"beta": 2, "alpha": true},
            "kind": "strategy"
        });
        assert_eq!(
            canonical_json(&value),
            "{\"kind\":\"strategy\",\"payload\":{\"alpha\":true,\"beta\":2},\"version\":\"1.0.0\"}\n"
        );
    }

    #[test]
    fn input_key_order_does_not_change_the_bytes() {
        let left: Value = serde_json::from_str(r#"{"b":1,"a":{"d":4,"c":3}}"#).expect("valid JSON");
        let right: Value =
            serde_json::from_str(r#"{"a":{"c":3,"d":4},"b":1}"#).expect("valid JSON");
        assert_eq!(canonical_json(&left), canonical_json(&right));
    }

    #[test]
    fn strings_are_escaped_and_round_trip() {
        let value = json!({"text": "a\"b\\c\nd\t\u{0001}"});
        let canonical = canonical_json(&value);
        let parsed: Value = serde_json::from_str(&canonical).expect("canonical JSON is valid");
        assert_eq!(parsed, value);
        assert!(canonical.contains("\\\""));
        assert!(canonical.contains("\\\\"));
        assert!(canonical.contains("\\n"));
        assert!(canonical.contains("\\u0001"));
    }

    #[test]
    fn the_stored_integer_and_float_forms_stay_distinct() {
        let value: Value =
            serde_json::from_str(r#"{"integer":2,"float":2.0}"#).expect("valid JSON");
        assert_eq!(canonical_json(&value), "{\"float\":2.0,\"integer\":2}\n");
    }

    #[test]
    fn the_result_ends_with_exactly_one_newline() {
        let canonical = canonical_json(&json!({}));
        assert_eq!(canonical, "{}\n");
        assert!(!canonical.ends_with("\n\n"));
    }
}
