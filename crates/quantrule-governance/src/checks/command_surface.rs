//! Check that the documented command surface matches the implemented shell.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

mod invoke;
mod lines;
mod usage;

use invoke::invocations_text;
use lines::{backticked, fenced_blocks, section, table_cells};

const RETIRED: [&str; 15] = [
    "candles",
    "candles-tolerant",
    "timeframes",
    "fetch",
    "events",
    "report model",
    "strategy explain",
    "strategy run",
    "execution run",
    "portfolio replay",
    "backtest run",
    "qualify",
    "analytics evaluate",
    "run check",
    "memory",
];

/// Check the command surface.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    match run(root) {
        Ok(diagnostics) => diagnostics,
        Err(message) => vec![Diagnostic::path(
            "command-surface",
            &root.join("README.md"),
            root,
            message,
        )],
    }
}

struct Help {
    usage: String,
}

fn run(root: &Path) -> Result<Vec<Diagnostic>, String> {
    let readme_path = root.join("README.md");
    let surface_path = root.join("docs/architecture/command-surface.md");
    let implemented = help_commands(root)?;
    let readme = readme_commands(root)?;
    let registry = surface_commands(root)?;
    let mut diagnostics = Vec::new();
    let names: BTreeSet<String> = implemented.keys().cloned().collect();
    let readme_set: BTreeSet<String> = readme.iter().cloned().collect();

    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for name in &readme {
        if !seen.insert(name.clone()) {
            duplicates.insert(name.clone());
        }
    }
    for name in duplicates {
        diagnostics.push(Diagnostic::path(
            "command-surface",
            &readme_path,
            root,
            format!("README.md documents '{name}' twice"),
        ));
    }
    compare(
        &mut diagnostics,
        root,
        &readme_path,
        "README.md",
        &names,
        &readme_set,
    );

    let registry_implemented: BTreeSet<String> = registry
        .iter()
        .filter(|(_, status)| *status == "Implemented")
        .map(|(name, _)| name.clone())
        .collect();
    let registry_planned: BTreeSet<String> = registry
        .iter()
        .filter(|(_, status)| *status == "Planned")
        .map(|(name, _)| name.clone())
        .collect();
    compare(
        &mut diagnostics,
        root,
        &surface_path,
        "command-surface.md",
        &names,
        &registry_implemented,
    );

    for name in registry_planned.intersection(&names) {
        diagnostics.push(Diagnostic::path(
            "command-surface",
            &surface_path,
            root,
            format!("command-surface.md marks implemented command '{name}' as Planned"),
        ));
    }
    for name in registry_planned.intersection(&readme_set) {
        diagnostics.push(Diagnostic::path(
            "command-surface",
            &readme_path,
            root,
            format!("README.md lists planned command '{name}' as available"),
        ));
    }

    let found = invoke::invocations(root, &names);
    let tutorials = root.join("docs/tutorials");
    for name in &names {
        if !found.get(name).is_some_and(|files| !files.is_empty()) {
            diagnostics.push(Diagnostic::path(
                "command-surface",
                &tutorials,
                root,
                format!("no tutorial code block invokes implemented command '{name}'"),
            ));
        }
    }

    for problem in tutorial_unimplemented(root, &names)? {
        diagnostics.push(problem);
    }
    for problem in document_unimplemented(root, &names)? {
        diagnostics.push(problem);
    }

    let grammar_files = current_documents(root)?;
    let grammar_tutorials = current_tutorials(root)?;
    let mut all = grammar_files;
    all.extend(grammar_tutorials);
    for problem in usage::invocation_errors(root, &implemented, &all)? {
        diagnostics.push(problem);
    }

    for problem in retired_config_commands(root)? {
        diagnostics.push(problem);
    }
    for problem in retired_architecture_commands(root)? {
        diagnostics.push(problem);
    }
    Ok(diagnostics)
}

fn help_commands(root: &Path) -> Result<BTreeMap<String, Help>, String> {
    let path = root.join("crates/quantrule-shell/src/command/help.rs");
    let text = read(&path, root)?;
    let start = text
        .find("const COMMANDS")
        .ok_or_else(|| "help.rs has no COMMANDS registry".to_owned())?;
    let end = text[start..]
        .find("];")
        .map(|offset| start + offset)
        .ok_or_else(|| "help.rs has an unterminated COMMANDS registry".to_owned())?;
    let block = &text[start..end];
    let starts: Vec<usize> = entry_starts(block);
    let mut commands = BTreeMap::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(block.len());
        let entry = &block[*start..end];
        let fields = fields(entry);
        let missing: Vec<&str> = ["name", "usage", "summary", "output"]
            .into_iter()
            .filter(|key| !fields.get(*key).is_some_and(|value| !value.is_empty()))
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "a CommandHelp entry is missing {}",
                missing.join(", ")
            ));
        }
        let name = fields["name"].clone();
        if commands.contains_key(&name) {
            return Err(format!("help.rs documents '{name}' twice"));
        }
        let usage = fields["usage"].clone();
        let name_tokens: Vec<&str> = name.split_whitespace().collect();
        let usage_tokens: Vec<&str> = usage.split_whitespace().collect();
        if usage_tokens.len() < name_tokens.len()
            || usage_tokens[..name_tokens.len()] != name_tokens
        {
            return Err(format!(
                "help.rs usage for '{name}' does not start with the command name: {usage:?}"
            ));
        }
        commands.insert(name, Help { usage });
    }
    if commands.is_empty() {
        return Err("help.rs documents no commands".to_owned());
    }
    Ok(commands)
}

fn entry_starts(block: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut search = 0;
    while let Some(index) = block[search..].find("CommandHelp") {
        let absolute = search + index;
        let mut rest = &block[absolute + "CommandHelp".len()..];
        rest = rest.trim_start();
        if rest.starts_with('{') {
            starts.push(absolute);
        }
        search = absolute + "CommandHelp".len();
    }
    starts
}

fn fields(entry: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    for line in entry.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if !matches!(key, "name" | "usage" | "summary" | "output") {
            continue;
        }
        let value = value.trim();
        let Some(value) = value.strip_prefix('"') else {
            continue;
        };
        let Some(end) = value.find('"') else {
            continue;
        };
        let tail = value[end + 1..].trim();
        if tail != "," {
            continue;
        }
        fields.insert(key.to_owned(), value[..end].to_owned());
    }
    fields
}

fn readme_commands(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("README.md");
    let text = read(&path, root)?;
    let body = section(root, &text, "Commands")?;
    let mut names = Vec::new();
    for line in body.lines() {
        let Some(rest) = line.strip_prefix('|') else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('`') else {
            continue;
        };
        let Some(end) = rest.find('`') else {
            continue;
        };
        let name = &rest[..end];
        if name.is_empty() || !rest[end + 1..].trim_start().starts_with('|') {
            continue;
        }
        names.push(name.to_owned());
    }
    if names.is_empty() {
        return Err("README.md has no command table under a Commands heading".to_owned());
    }
    Ok(names)
}

fn surface_commands(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let path = root.join("docs/architecture/command-surface.md");
    let text = read(&path, root)?;
    let body = section(root, &text, "Command registry")?;
    let lines: Vec<&str> = body.lines().collect();
    let Some(header) = lines
        .iter()
        .position(|line| line.trim().starts_with("| Command |"))
    else {
        return Err(
            "command-surface.md has no '| Command |' table under Command registry".to_owned(),
        );
    };
    let columns = table_cells(lines[header]);
    if !columns.iter().any(|column| column == "Command")
        || !columns.iter().any(|column| column == "Status")
    {
        return Err(
            "command-surface.md registry table needs Command and Status columns".to_owned(),
        );
    }
    let mut statuses = BTreeMap::new();
    for line in lines.iter().skip(header + 2) {
        if !line.trim().starts_with('|') {
            break;
        }
        let cells = table_cells(line);
        if cells.len() != columns.len() {
            return Err(format!(
                "command-surface.md registry row has the wrong column count: {line}"
            ));
        }
        let row: BTreeMap<&str, &str> = columns
            .iter()
            .map(String::as_str)
            .zip(cells.iter().map(String::as_str))
            .collect();
        let command = row
            .get("Command")
            .map(|value| value.trim().trim_matches('`'))
            .unwrap_or("");
        let status = *row.get("Status").unwrap_or(&"");
        if statuses.contains_key(command) {
            return Err(format!("command-surface.md documents '{command}' twice"));
        }
        if status != "Implemented" && status != "Planned" {
            return Err(format!(
                "command-surface.md status for '{command}' must be Implemented or Planned: {status:?}"
            ));
        }
        statuses.insert(command.to_owned(), status.to_owned());
    }
    if statuses.is_empty() {
        return Err("command-surface.md has an empty command registry".to_owned());
    }
    Ok(statuses)
}

fn current_status(root: &Path, path: &Path) -> bool {
    let text = read(path, root).unwrap_or_default();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("- Status:") else {
            continue;
        };
        let Some(status) = rest.split_whitespace().next() else {
            return true;
        };
        return !["Historical", "Deprecated", "Superseded"]
            .iter()
            .any(|prefix| status.starts_with(prefix));
    }
    true
}

fn retired_backticks(root: &Path, files: &[PathBuf]) -> Vec<Diagnostic> {
    let mut problems = Vec::new();
    for path in files {
        if !current_status(root, path) {
            continue;
        }
        let text = read(path, root).unwrap_or_default();
        for (number, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if ["removed", "retired", "not current", "historical"]
                .iter()
                .any(|word| lower.contains(word))
            {
                continue;
            }
            for value in backticked(line) {
                if RETIRED.contains(&value.as_str()) {
                    problems.push(Diagnostic::line(
                        "command-surface",
                        path,
                        root,
                        number + 1,
                        format!("retired command {value:?} in a current contract"),
                    ));
                }
            }
        }
    }
    problems
}

fn retired_config_commands(root: &Path) -> Result<Vec<Diagnostic>, String> {
    let directory = root.join("docs/config");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect(&directory, "md", &mut files);
    files.sort();
    Ok(retired_backticks(root, &files))
}

fn retired_architecture_commands(root: &Path) -> Result<Vec<Diagnostic>, String> {
    let directory = root.join("docs/architecture");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_some_and(|extension| extension == "md")
            && path.is_file()
            && !matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("command-surface.md" | "client.md")
            )
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(retired_backticks(root, &files))
}

fn tutorial_unimplemented(
    root: &Path,
    commands: &BTreeSet<String>,
) -> Result<Vec<Diagnostic>, String> {
    let directory = root.join("docs/tutorials");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect(&directory, "md", &mut files);
    files.sort();
    let mut problems = Vec::new();
    for path in files {
        for block in fenced_blocks(root, &path)? {
            for rest in invocations_text(&block) {
                if rest == "quit" || rest.starts_with("quit ") {
                    continue;
                }
                if !commands
                    .iter()
                    .any(|command| rest == *command || rest.starts_with(&format!("{command} ")))
                {
                    problems.push(Diagnostic::path(
                        "command-surface",
                        &path,
                        root,
                        format!("{rest:?}"),
                    ));
                }
            }
        }
    }
    Ok(problems)
}

fn document_unimplemented(
    root: &Path,
    commands: &BTreeSet<String>,
) -> Result<Vec<Diagnostic>, String> {
    let mut problems = Vec::new();
    for path in current_documents(root)? {
        for block in fenced_blocks(root, &path)? {
            for rest in invocations_text(&block) {
                if rest == "quit" || rest.starts_with("quit ") {
                    continue;
                }
                if !commands
                    .iter()
                    .any(|command| rest == *command || rest.starts_with(&format!("{command} ")))
                {
                    problems.push(Diagnostic::path(
                        "command-surface",
                        &path,
                        root,
                        format!("{rest:?}"),
                    ));
                }
            }
        }
    }
    Ok(problems)
}

fn current_documents(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = vec![root.join("AGENTS.md")];
    let workflows = root.join("docs/workflows");
    if workflows.is_dir() {
        for entry in fs::read_dir(&workflows).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().is_some_and(|extension| extension == "md") && path.is_file() {
                files.push(path);
            }
        }
    }
    let config = root.join("docs/config");
    if config.is_dir() {
        let mut found = Vec::new();
        collect(&config, "md", &mut found);
        found.sort();
        files.extend(found);
    }
    files.retain(|path| path.is_file() && current_status(root, path));
    Ok(files)
}

fn current_tutorials(root: &Path) -> Result<Vec<PathBuf>, String> {
    let directory = root.join("docs/tutorials");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect(&directory, "md", &mut files);
    files.sort();
    files.retain(|path| current_status(root, path));
    Ok(files)
}

fn compare(
    diagnostics: &mut Vec<Diagnostic>,
    root: &Path,
    path: &Path,
    label: &str,
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
) {
    for name in expected.difference(actual) {
        diagnostics.push(Diagnostic::path(
            "command-surface",
            path,
            root,
            format!("{label} is missing implemented command '{name}'"),
        ));
    }
    for name in actual.difference(expected) {
        diagnostics.push(Diagnostic::path(
            "command-surface",
            path,
            root,
            format!("{label} lists '{name}', which help.rs does not implement"),
        ));
    }
}

fn read(path: &Path, root: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", relative(root, path)))
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect(directory: &Path, extension: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, extension, found);
        } else if path.extension().is_some_and(|value| value == extension) {
            found.push(path);
        }
    }
}

fn problem(root: &Path, path: &Path, message: String) -> Diagnostic {
    Diagnostic::path("command-surface", path, root, message)
}

#[cfg(test)]
mod tests {
    use crate::checks::command_surface::invoke::contains_invocation;
    use crate::checks::command_surface::lines::backticked;
    use crate::checks::command_surface::usage::usage_shape;

    #[test]
    fn a_retired_command_is_found_between_backticks() {
        let values = backticked("run `backtest run` now");
        assert_eq!(values, vec!["backtest run"]);
    }

    #[test]
    fn a_usage_line_yields_its_shape() {
        let shape = usage_shape("run submit <config> [--name <value>]", "run submit");
        assert_eq!(shape.required_positionals, 1);
        assert_eq!(shape.max_positionals, 1);
        assert!(shape.required_options.is_empty());
        assert!(shape.optional_options.contains("name"));
    }

    #[test]
    fn an_invocation_matches_whole_command_tokens() {
        assert!(contains_invocation(
            "quantrule run submit x\n",
            "run submit"
        ));
        assert!(!contains_invocation(
            "quantrule run submission x\n",
            "run submit"
        ));
    }
}
