//! Small text scanners shared by the config check rows.
//!
//! HAProxy sections and rendered Compose service blocks are delimited by
//! indentation and by the next section header. The helpers here return the
//! empty string when the header is absent, so a missing unit is a row failure
//! rather than a substring match against a mount path or an alias.

/// One named HAProxy section, from its header to the next `frontend`, `backend`,
/// or `listen` header. The header includes its leading newline, so
/// `default_backend` does not match a backend header. An absent header yields
/// the empty string.
pub(super) fn section<'a>(text: &'a str, header: &str) -> &'a str {
    let Some(start) = text.find(header) else {
        return "";
    };
    let rest = &text[start..];
    let end = rest[1..]
        .find("\nfrontend ")
        .into_iter()
        .chain(rest[1..].find("\nbackend "))
        .chain(rest[1..].find("\nlisten "))
        .min()
        .map_or(rest.len(), |index| index + 1);
    &rest[..end]
}

/// One 2-space-indented Compose service block. The header includes its leading
/// newline so a mount path or a network alias cannot match it.
pub(super) fn service_block<'a>(text: &'a str, key: &str) -> &'a str {
    let header = format!("\n  {key}:\n");
    let Some(start) = text.find(&header) else {
        return "";
    };
    let rest = &text[start + header.len()..];
    let end = rest
        .match_indices("\n  ")
        .find(|(index, _)| {
            let after = &rest[index + 3..];
            !after.starts_with(' ') && !after.starts_with('\n')
        })
        .map_or(rest.len(), |(index, _)| index);
    &rest[..end]
}
