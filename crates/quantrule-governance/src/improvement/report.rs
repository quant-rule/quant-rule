//! Python-compatible JSON rendering for improvement reports.

/// One JSON value.
pub enum Value {
    /// JSON null.
    Null,
    /// JSON boolean.
    Bool(bool),
    /// JSON number, including the non-finite values Python writes.
    Number(f64),
    /// JSON string.
    String(String),
    /// JSON array.
    Array(Vec<Value>),
    /// JSON object; keys are sorted when rendered.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Render with two-space indentation, sorted keys, and a trailing newline.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        write(self, 0, &mut text);
        text.push('\n');
        text
    }
}

fn write(value: &Value, level: usize, text: &mut String) {
    match value {
        Value::Null => text.push_str("null"),
        Value::Bool(value) => text.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) if value.is_finite() => {
            text.push_str(
                &serde_json::Number::from_f64(*value)
                    .map_or_else(|| value.to_string(), |number| number.to_string()),
            );
        }
        Value::Number(value) if value.is_nan() => text.push_str("NaN"),
        Value::Number(value) if value.is_sign_negative() => text.push_str("-Infinity"),
        Value::Number(_) => text.push_str("Infinity"),
        Value::String(value) => text.push_str(&serde_json::to_string(value).unwrap_or_default()),
        Value::Array(items) if items.is_empty() => text.push_str("[]"),
        Value::Array(items) => {
            text.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                text.push_str(&"  ".repeat(level + 1));
                write(item, level + 1, text);
                if index + 1 < items.len() {
                    text.push(',');
                }
                text.push('\n');
            }
            text.push_str(&"  ".repeat(level));
            text.push(']');
        }
        Value::Object(entries) if entries.is_empty() => text.push_str("{}"),
        Value::Object(entries) => {
            let mut entries: Vec<&(String, Value)> = entries.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            text.push_str("{\n");
            for (index, (key, value)) in entries.iter().enumerate() {
                text.push_str(&"  ".repeat(level + 1));
                text.push_str(&serde_json::to_string(key).unwrap_or_default());
                text.push_str(": ");
                write(value, level + 1, text);
                if index + 1 < entries.len() {
                    text.push(',');
                }
                text.push('\n');
            }
            text.push_str(&"  ".repeat(level));
            text.push('}');
        }
    }
}
