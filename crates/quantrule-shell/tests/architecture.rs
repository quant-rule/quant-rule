//! Architecture guards for the shell crate.
//!
//! The shell parses, calls the pipeline, and renders. These source-level checks
//! protect the one-way renderer edges and the single strategy walk.

use std::fs;
use std::path::{Path, PathBuf};

fn source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn sources(directory: &str) -> Vec<(PathBuf, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(directory);
    let mut files = Vec::new();
    collect(&root, &mut files);
    files
}

fn collect(path: &Path, files: &mut Vec<(PathBuf, String)>) {
    let entries =
        fs::read_dir(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|error| panic!("read entry in {}: {error}", path.display()));
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            files.push((path, text));
        }
    }
}

#[test]
fn renderers_do_not_depend_on_the_command_module() {
    for file in ["lab_summary.rs", "render_state.rs"] {
        let text = source(file);
        assert!(
            !text.contains("use crate::command"),
            "{file} must not import command; the command module calls the renderer"
        );
    }
}

#[test]
fn the_shell_does_not_walk_the_strategy() {
    for (path, text) in sources("command") {
        assert!(
            !text.contains(".on_bar("),
            "{} must not call the strategy walk; the runner owns it",
            path.display()
        );
    }
}

#[test]
fn commands_do_not_rebuild_the_ladder_or_model_inputs() {
    for (path, text) in sources("command") {
        assert!(
            !text.contains("TimeframeInput {"),
            "{} must use the pipeline input builder",
            path.display()
        );
        assert!(
            !text.contains("const LADDER"),
            "{} must use the data engine ladder",
            path.display()
        );
    }
}
