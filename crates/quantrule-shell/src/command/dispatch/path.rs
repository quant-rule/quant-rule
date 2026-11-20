//! Shared path, query, and profile-slug helpers.

use crate::error::RunError;

pub(super) fn slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut pending_hyphen = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.push(character.to_ascii_lowercase());
        } else {
            pending_hyphen = true;
        }
    }
    if slug.len() > 64 {
        slug.truncate(64);
        while slug.ends_with('-') {
            slug.pop();
        }
    }
    slug
}

pub(super) fn with_query(path: &str, query: &[String]) -> String {
    if query.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", query.join("&"))
    }
}

pub(super) fn safe_segments(name: &str) -> Result<Vec<&str>, RunError> {
    if name.is_empty() || name.starts_with('/') || name.contains('\\') {
        return Err(unsafe_name(name));
    }
    let segments: Vec<&str> = name.split('/').collect();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err(unsafe_name(name));
    }
    Ok(segments)
}

fn unsafe_name(name: &str) -> RunError {
    RunError::Decode {
        path: String::from("a run artifact"),
        message: format!("the artifact name '{name}' is not a safe relative path"),
    }
}

#[cfg(test)]
mod tests {
    use super::{safe_segments, slug};

    #[test]
    fn a_slug_is_lowercase_and_hyphenated() {
        assert_eq!(slug("Research 2026"), "research-2026");
        assert_eq!(slug("  --Alpha-- "), "alpha");
        assert_eq!(slug(""), "");
    }

    #[test]
    fn a_slug_is_capped_at_sixty_four_characters() {
        let slug = slug(&"a".repeat(80));
        assert_eq!(slug.len(), 64);
    }

    #[test]
    fn a_relative_artifact_path_is_accepted() {
        assert_eq!(
            safe_segments("tables/equity.csv").expect("safe"),
            vec!["tables", "equity.csv"]
        );
    }

    #[test]
    fn a_traversal_artifact_path_is_refused() {
        for name in ["../etc/passwd", "/etc/passwd", "a//b", "a/./b", "a\\b", ""] {
            assert!(safe_segments(name).is_err(), "{name}");
        }
    }
}
