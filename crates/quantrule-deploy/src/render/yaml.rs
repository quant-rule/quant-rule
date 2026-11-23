//! A small deterministic YAML emitter for the v3 renderer.
//!
//! The renderer would otherwise need a YAML dependency, and every YAML library
//! makes its own choice about quoting, key order, and scalar folding. This
//! emitter has one rule: every string is a JSON double-quoted scalar, every map
//! is sorted, and every list keeps its constructed order. The output is valid
//! YAML 1.2 and therefore valid Compose input.

use std::collections::BTreeMap;

/// One YAML node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yaml {
    /// A boolean scalar.
    Bool(bool),
    /// An unsigned integer scalar.
    Number(u64),
    /// A string scalar.
    String(String),
    /// A block sequence.
    List(Vec<Yaml>),
    /// A block mapping with sorted keys.
    Map(BTreeMap<String, Yaml>),
}

impl Yaml {
    /// An empty mapping.
    #[must_use]
    pub fn map() -> Self {
        Self::Map(BTreeMap::new())
    }

    /// A mapping with one entry per pair.
    #[must_use]
    pub fn mapping(entries: impl IntoIterator<Item = (String, Yaml)>) -> Self {
        Self::Map(entries.into_iter().collect())
    }

    /// A string scalar.
    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// An unsigned integer scalar.
    #[must_use]
    pub fn number(value: impl Into<u64>) -> Self {
        Self::Number(value.into())
    }

    /// A sequence.
    #[must_use]
    pub fn list(values: impl IntoIterator<Item = Yaml>) -> Self {
        Self::List(values.into_iter().collect())
    }

    /// Insert one mapping entry.
    ///
    /// # Panics
    ///
    /// Panics only when called on a non-mapping node. The renderer builds
    /// mappings explicitly, so this is a programming error rather than input.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: Yaml) -> Self {
        if let Self::Map(entries) = &mut self {
            entries.insert(key.into(), value);
        } else {
            panic!("Yaml::with requires a mapping");
        }
        self
    }

    /// Render the node as a YAML document.
    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::new();
        emit(self, 0, &mut output);
        output
    }
}

fn emit(value: &Yaml, indent: usize, output: &mut String) {
    match value {
        Yaml::Map(entries) if entries.is_empty() => {
            write_indent(indent, output);
            output.push_str("{}\n");
        }
        Yaml::Map(entries) => {
            for (name, child) in entries {
                write_indent(indent, output);
                output.push_str(&plain_or_quoted_key(name));
                output.push(':');
                emit_child(child, indent, output);
            }
        }
        Yaml::List(values) if values.is_empty() => {
            write_indent(indent, output);
            output.push_str("[]\n");
        }
        Yaml::List(values) => {
            for child in values {
                write_indent(indent, output);
                output.push_str("-\n");
                emit(child, indent + 2, output);
            }
        }
        scalar => {
            write_indent(indent, output);
            output.push_str(&scalar_text(scalar));
            output.push('\n');
        }
    }
}

fn emit_child(child: &Yaml, indent: usize, output: &mut String) {
    match child {
        Yaml::Map(entries) if entries.is_empty() => output.push_str(" {}\n"),
        Yaml::List(values) if values.is_empty() => output.push_str(" []\n"),
        Yaml::Map(_) | Yaml::List(_) => {
            output.push('\n');
            emit(child, indent + 2, output);
        }
        scalar => {
            output.push(' ');
            output.push_str(&scalar_text(scalar));
            output.push('\n');
        }
    }
}

fn scalar_text(value: &Yaml) -> String {
    match value {
        Yaml::Bool(value) => value.to_string(),
        Yaml::Number(value) => value.to_string(),
        Yaml::String(value) => quote(value),
        Yaml::List(_) | Yaml::Map(_) => String::new(),
    }
}

fn plain_or_quoted_key(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        value.to_owned()
    } else {
        quote(value)
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

fn write_indent(indent: usize, output: &mut String) {
    for _ in 0..indent {
        output.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Yaml {
        Yaml::map().with("version", Yaml::string("3.9")).with(
            "services",
            Yaml::map().with(
                "gateway",
                Yaml::map()
                    .with("image", Yaml::string("registry/server@sha256:abc"))
                    .with("replicas", Yaml::number(2_u64))
                    .with("enabled", Yaml::Bool(true))
                    .with(
                        "labels",
                        Yaml::list([Yaml::string("a: b"), Yaml::string("plain")]),
                    ),
            ),
        )
    }

    #[test]
    fn mapping_keys_are_sorted_and_strings_are_quoted() {
        let text = sample().render();
        assert_eq!(
            text,
            concat!(
                "services:\n",
                "  gateway:\n",
                "    enabled: true\n",
                "    image: \"registry/server@sha256:abc\"\n",
                "    labels:\n",
                "      -\n",
                "        \"a: b\"\n",
                "      -\n",
                "        \"plain\"\n",
                "    replicas: 2\n",
                "version: \"3.9\"\n",
            )
        );
    }

    #[test]
    fn empty_containers_are_explicit() {
        let text = Yaml::map()
            .with("empty_map", Yaml::map())
            .with("empty_list", Yaml::list(Vec::<Yaml>::new()))
            .render();
        assert_eq!(text, "empty_list: []\nempty_map: {}\n");
    }

    #[test]
    fn the_same_tree_renders_the_same_bytes() {
        let first = sample().render();
        let second = sample().render();
        assert_eq!(first, second);
    }

    #[test]
    fn a_key_with_a_colon_is_quoted() {
        let text = Yaml::map().with("a: b", Yaml::string("value")).render();
        assert_eq!(text, "\"a: b\": \"value\"\n");
    }
}
