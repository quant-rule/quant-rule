//! Source-level guards for the native command bridge.
//!
//! The Tauri command list, the Rust command functions, the frontend `call`
//! names, and the browser fixture cases must agree. A source-level test catches
//! a missing registration or a renamed command without needing a display; the
//! full native WebDriver E2E remains a separate gate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn app_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("the desktop app root resolves")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn registered_commands(lib: &str) -> BTreeSet<String> {
    let start = lib
        .find("generate_handler![")
        .expect("the command handler list exists");
    let rest = &lib[start..];
    let end = rest.find("])").expect("the command handler list closes");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let name = line.strip_prefix("commands::")?;
            Some(name.trim_end_matches(',').to_owned())
        })
        .collect()
}

fn command_functions(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = source;
    while let Some(index) = rest.find("#[tauri::command") {
        let after = &rest[index..];
        let Some(function) = after.find("pub fn ") else {
            break;
        };
        let after = &after[function + "pub fn ".len()..];
        let name: String = after
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        let length = name.len();
        if !name.is_empty() {
            names.insert(name);
        }
        rest = &after[length..];
    }
    names
}

fn frontend_calls(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = source;
    while let Some(index) = rest.find("call") {
        let after = &rest[index + "call".len()..];
        // Skip the function definition and type-only mentions.
        let Some(quote) = after.find('"') else {
            break;
        };
        let before_quote = &after[..quote];
        if !before_quote.trim_start().starts_with('(')
            && !before_quote.trim_start().starts_with("::<")
            && !before_quote.trim_start().starts_with('<')
        {
            rest = &after[quote + 1..];
            continue;
        }
        let after_quote = &after[quote + 1..];
        let Some(end) = after_quote.find('"') else {
            break;
        };
        let name = &after_quote[..end];
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            names.insert(name.to_owned());
        }
        rest = &after_quote[end + 1..];
    }
    names
}

fn mock_cases(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = source;
    while let Some(index) = rest.find("case \"") {
        let after = &rest[index + "case \"".len()..];
        let Some(end) = after.find('"') else {
            break;
        };
        let name = &after[..end];
        if name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            names.insert(name.to_owned());
        }
        rest = &after[end + 1..];
    }
    names
}

fn command_sources(directory: &Path) -> String {
    let mut files = Vec::new();
    collect_files(directory, &mut files);
    let mut source = String::new();
    for file in files {
        if file.extension().is_some_and(|extension| extension == "rs") {
            source.push_str(&read(&file));
            source.push('\n');
        }
    }
    source
}

#[test]
fn every_registered_command_is_a_command_function_and_vice_versa() {
    let root = app_root();
    let lib = read(&root.join("src-tauri/src/lib.rs"));
    let commands = command_sources(&root.join("src-tauri/src/commands"));
    let registered = registered_commands(&lib);
    let functions = command_functions(&commands);
    assert_eq!(
        registered, functions,
        "the Tauri handler list and the command functions must agree"
    );
}

#[test]
fn every_frontend_call_and_mock_case_is_registered() {
    let root = app_root();
    let lib = read(&root.join("src-tauri/src/lib.rs"));
    let registered = registered_commands(&lib);
    let mut files = Vec::new();
    collect_files(&root.join("src"), &mut files);
    let mut calls = BTreeSet::new();
    for file in files {
        if file.extension().is_some_and(|extension| extension == "ts")
            || file.extension().is_some_and(|extension| extension == "svelte")
        {
            calls.extend(frontend_calls(&read(&file)));
        }
    }
    let mock = read(&root.join("src/lib/mock.ts"));
    calls.extend(mock_cases(&mock));
    for call in &calls {
        assert!(
            registered.contains(call),
            "frontend command {call} is not registered in lib.rs"
        );
    }
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
    for entry in entries {
        let entry = entry.expect("the directory entry is readable");
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}
