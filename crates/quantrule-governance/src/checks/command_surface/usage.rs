//! Parse usage lines and validate documented invocations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

use super::lines::fenced_blocks;
use super::{Help, problem};

#[derive(Default)]
pub(super) struct Shape {
    pub(super) required_positionals: usize,
    pub(super) max_positionals: usize,
    pub(super) required_options: BTreeSet<String>,
    pub(super) optional_options: BTreeSet<String>,
}
pub(super) fn usage_shape(usage: &str, name: &str) -> Shape {
    let rest = usage.strip_prefix(name).unwrap_or(usage).trim();
    let mut tokens: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    let mut depth: usize = 0;
    let mut optional = false;
    for character in rest.chars() {
        match character {
            '[' => {
                depth += 1;
                current.push(character);
                optional = true;
            }
            ']' => {
                current.push(character);
                depth = depth.saturating_sub(1);
            }
            value if value.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push((std::mem::take(&mut current), optional));
                    optional = false;
                }
            }
            _ => {
                if current.is_empty() && depth > 0 {
                    optional = true;
                }
                current.push(character);
            }
        }
    }
    if !current.is_empty() {
        tokens.push((current, optional || depth > 0));
    }

    let mut required_positionals = 0;
    let mut optional_positionals = 0;
    let mut required_options = BTreeSet::new();
    let mut optional_options = BTreeSet::new();
    let mut index = 0;
    while index < tokens.len() {
        let (raw, optional) = &tokens[index];
        let token = raw.trim_matches(|ch| ch == '[' || ch == ']');
        if let Some(option) = token.strip_prefix("--") {
            let option = option.split('=').next().unwrap_or("");
            let next = tokens
                .get(index + 1)
                .map(|(value, _)| value.trim_matches(|ch| ch == '[' || ch == ']'))
                .unwrap_or("");
            let takes_value = !next.is_empty() && !next.starts_with("--");
            if *optional {
                optional_options.insert(option.to_owned());
            } else {
                required_options.insert(option.to_owned());
            }
            if takes_value {
                index += 2;
                continue;
            }
        } else if token.starts_with('<')
            || token.contains('|')
            || matches!(token, "all" | "glossary" | "workflows")
        {
            if *optional {
                optional_positionals += 1;
            } else {
                required_positionals += 1;
            }
        }
        index += 1;
    }
    let mut max_positionals = required_positionals + optional_positionals;
    if name == "help" {
        max_positionals = max_positionals.max(2);
    }
    Shape {
        required_positionals,
        max_positionals,
        required_options,
        optional_options,
    }
}

pub(super) fn invocation_errors(
    root: &Path,
    commands: &BTreeMap<String, Help>,
    files: &[PathBuf],
) -> Result<Vec<Diagnostic>, String> {
    let mut problems = Vec::new();
    let global_value_options = ["profile", "token"];
    let global_flags = ["json"];
    for path in files {
        for block in fenced_blocks(root, path)? {
            let mut logical = Vec::new();
            let mut pending = String::new();
            for line in block.split('\n') {
                let part = line.trim();
                if pending.is_empty() {
                    pending = part.to_owned();
                } else {
                    pending.push(' ');
                    pending.push_str(part);
                }
                if pending.ends_with('\\') {
                    pending.pop();
                    pending = pending.trim_end().to_owned();
                    continue;
                }
                logical.push(std::mem::take(&mut pending));
            }
            if !pending.is_empty() {
                logical.push(pending);
            }
            for stripped in logical {
                if !stripped.starts_with("quantrule ") {
                    continue;
                }
                let words = match shlex_split(&stripped) {
                    Ok(words) => words,
                    Err(error) => {
                        problems.push(problem(
                            root,
                            path,
                            format!("cannot parse {stripped:?}: {error}"),
                        ));
                        continue;
                    }
                };
                if words.len() < 2 {
                    continue;
                }
                let Some((matched, help)) = match_command(commands, &words) else {
                    continue;
                };
                let shape = usage_shape(&help.usage, matched);
                let args = &words[1 + matched.split_whitespace().count()..];
                let mut positionals = 0;
                let mut found_options = BTreeSet::new();
                let mut index = 0;
                while index < args.len() {
                    let token = &args[index];
                    if let Some(option) = token.strip_prefix("--") {
                        let option = option.split('=').next().unwrap_or("");
                        if global_flags.contains(&option) {
                            index += 1;
                            continue;
                        }
                        if global_value_options.contains(&option) {
                            if !token.contains('=') {
                                if index + 1 >= args.len() {
                                    problems.push(problem(
                                        root,
                                        path,
                                        format!("--{option} has no value in {stripped:?}"),
                                    ));
                                }
                                index += 2;
                            } else {
                                index += 1;
                            }
                            continue;
                        }
                        if !shape.required_options.contains(option)
                            && !shape.optional_options.contains(option)
                        {
                            problems.push(problem(
                                root,
                                path,
                                format!("unknown option '--{option}' in {stripped:?}"),
                            ));
                            index += 1;
                            continue;
                        }
                        found_options.insert(option.to_owned());
                        let takes_value =
                            index + 1 < args.len() && !args[index + 1].starts_with("--");
                        index += if takes_value { 2 } else { 1 };
                        continue;
                    }
                    positionals += 1;
                    index += 1;
                }
                if positionals < shape.required_positionals || positionals > shape.max_positionals {
                    problems.push(problem(
                        root,
                        path,
                        format!(
                            "{stripped:?} has {positionals} positional(s); expected {}..{}",
                            shape.required_positionals, shape.max_positionals
                        ),
                    ));
                }
                let missing: Vec<String> = shape
                    .required_options
                    .difference(&found_options)
                    .map(|name| format!("--{name}"))
                    .collect();
                if !missing.is_empty() {
                    problems.push(problem(
                        root,
                        path,
                        format!(
                            "{stripped:?} is missing required option(s) {}",
                            missing.join(", ")
                        ),
                    ));
                }
            }
        }
    }
    Ok(problems)
}

fn shlex_split(text: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut has = false;
    let mut quote: Option<char> = None;
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match quote {
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                } else {
                    current.push(character);
                }
            }
            Some('"') => match character {
                '"' => quote = None,
                '\\' => match chars.peek().copied() {
                    Some(next) if matches!(next, '"' | '\\' | '$' | '`') => {
                        current.push(next);
                        chars.next();
                    }
                    Some('\n') => {
                        chars.next();
                    }
                    Some(_) | None => current.push('\\'),
                },
                _ => current.push(character),
            },
            Some(_) => current.push(character),
            None => match character {
                '\'' | '"' => {
                    quote = Some(character);
                    has = true;
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                        has = true;
                    }
                }
                value if value.is_whitespace() => {
                    if has {
                        words.push(std::mem::take(&mut current));
                        has = false;
                    }
                }
                _ => {
                    current.push(character);
                    has = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err("No closing quotation".to_owned());
    }
    if has {
        words.push(current);
    }
    Ok(words)
}

fn match_command<'a>(
    commands: &'a BTreeMap<String, Help>,
    words: &[String],
) -> Option<(&'a str, &'a Help)> {
    let mut names: Vec<&String> = commands.keys().collect();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    for name in names {
        let tokens: Vec<&str> = name.split_whitespace().collect();
        if words.len() > tokens.len()
            && words[1..1 + tokens.len()]
                .iter()
                .zip(&tokens)
                .all(|(word, token)| word == token)
        {
            return Some((name.as_str(), &commands[name]));
        }
    }
    None
}
