//! Ordered JSON rendering with the repository's two-space layout.

/// One ordered JSON value.
pub enum Json {
    /// A string.
    String(String),
    /// An ordered array.
    Array(Vec<Json>),
    /// An ordered object.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// A string value.
    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// An object value.
    #[must_use]
    pub fn object(entries: Vec<(String, Self)>) -> Self {
        Self::Object(entries)
    }

    /// Render with two-space indentation and a trailing newline.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        write(self, 0, &mut text);
        text.push('\n');
        text
    }
}

fn write(value: &Json, level: usize, text: &mut String) {
    match value {
        Json::String(value) => text.push_str(&serde_json::to_string(value).unwrap_or_default()),
        Json::Array(items) if items.is_empty() => text.push_str("[]"),
        Json::Array(items) => {
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
        Json::Object(entries) if entries.is_empty() => text.push_str("{}"),
        Json::Object(entries) => {
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
