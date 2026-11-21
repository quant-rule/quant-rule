//! Guard the release workflow's image order and concurrency key.

use std::fs;
use std::path::{Path, PathBuf};

const GROUP: &str = "release-${{ inputs.release_tag || github.ref_name }}";
const BUILD: &str = "Build the image";
const NO_PYTHON: &str = "Assert the image carries no Python";
const CONTRACT: &str = "Check the image contract";
const PUSH: &str = "Push the verified image";
const COMPOSED: &str = "Run the composed suite on the local image";
const CHECK_STEPS: [&str; 2] = [NO_PYTHON, CONTRACT];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn workflow() -> String {
    fs::read_to_string(root().join(".github/workflows/release.yml")).expect("release.yml")
}

fn job<'a>(text: &'a str, name: &str, next_name: &str) -> &'a str {
    let start = text.find(&format!("\n  {name}:\n"));
    let end = text.find(&format!("\n  {next_name}:\n"));
    match (start, end) {
        (Some(start), Some(end)) if end > start => &text[start..end],
        _ => "",
    }
}

fn image_job(text: &str) -> &str {
    let block = job(text, "image", "quality");
    if block.is_empty() {
        job(text, "image", "publish")
    } else {
        block
    }
}

fn steps(text: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    for block in text.split("\n      - name: ").skip(1) {
        let (name, body) = block.split_once('\n').unwrap_or((block, ""));
        found.push((name.trim(), body));
    }
    found
}

fn problems(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut group_line = None;
    let mut in_concurrency = false;
    for line in text.lines() {
        let stripped = line.trim();
        if line.starts_with("concurrency:") {
            in_concurrency = true;
            continue;
        }
        if in_concurrency {
            if stripped.is_empty() || stripped.starts_with('#') {
                continue;
            }
            if let Some(value) = stripped.strip_prefix("group:") {
                group_line = Some(value.trim().to_owned());
            }
            break;
        }
    }
    if group_line.as_deref() != Some(GROUP) {
        found.push(format!(
            "concurrency group is {group_line:?}, expected {GROUP:?}"
        ));
    }

    let image = image_job(text);
    if image.is_empty() {
        found.push("the image job is missing".to_owned());
        return found;
    }
    let order = steps(image);
    let names: Vec<&str> = order.iter().map(|(name, _)| *name).collect();
    for name in [BUILD, NO_PYTHON, CONTRACT, PUSH] {
        if !names.contains(&name) {
            found.push(format!("the image job has no {name:?} step"));
        }
    }
    if [BUILD, NO_PYTHON, CONTRACT, PUSH]
        .iter()
        .any(|name| !names.contains(name))
    {
        return found;
    }

    for name in [BUILD, NO_PYTHON, CONTRACT] {
        let body = order
            .iter()
            .find(|(step, _)| *step == name)
            .map(|(_, body)| *body)
            .unwrap_or("");
        if body.contains("docker push") {
            found.push(format!(
                "{name:?} publishes the image before the checks pass"
            ));
        }
    }
    let push = order
        .iter()
        .find(|(step, _)| *step == PUSH)
        .map(|(_, body)| *body)
        .unwrap_or("");
    if !push.contains("docker push") {
        found.push(format!("{PUSH:?} does not publish the image"));
    }
    let push_at = names.iter().position(|name| *name == PUSH).unwrap_or(0);
    for name in CHECK_STEPS {
        if names.iter().position(|step| *step == name).unwrap_or(0) > push_at {
            found.push(format!("{name:?} runs after the image is published"));
        }
    }
    match order.iter().find(|(step, _)| *step == COMPOSED) {
        None => found.push(format!("the image job has no {COMPOSED:?} step")),
        Some((_, body)) => {
            if body.contains("docker push") {
                found.push(format!(
                    "{COMPOSED:?} publishes the image before the suite passes"
                ));
            }
            if names.iter().position(|step| *step == COMPOSED).unwrap_or(0) > push_at {
                found.push(format!("{COMPOSED:?} runs after the image is published"));
            }
            if !body.contains("tools/test_tiers.sh composed") {
                found.push(format!("{COMPOSED:?} does not run the composed tier"));
            }
        }
    }

    if !image.contains("Dockerfile.release") {
        found.push("the image job does not use Dockerfile.release".to_owned());
    }
    if !image.contains("gh run download") || !image.contains("release-binaries-$GITHUB_SHA") {
        found.push("the image job does not download the exact-SHA release binaries".to_owned());
    }
    if image.contains("cargo build") {
        found.push("the image job compiles Rust instead of promoting prebuilt binaries".to_owned());
    }
    if text.contains("\n  desktop:\n") {
        found.push("the desktop job must be removed".to_owned());
    }

    let quality = job(text, "quality", "publish");
    if quality.is_empty() {
        found.push("the quality job is missing".to_owned());
    } else {
        for command in ["bun run check", "bun test", "bun run build", "bun run e2e"] {
            if !quality.contains(command) {
                found.push(format!("the quality job does not run {command:?}"));
            }
        }
    }
    let cli = job(text, "cli", "publish");
    if cli.is_empty() {
        found.push("the CLI release job is missing".to_owned());
    } else if !cli.contains("cargo test -p quantrule-shell") {
        found.push("the CLI release job does not test quantrule-shell".to_owned());
    }

    let publish = text
        .find("\n  publish:\n")
        .map(|index| &text[index..])
        .unwrap_or("");
    if !publish.contains("\n      - quality\n") {
        found.push("the publish job does not require the quality job".to_owned());
    }
    if !publish.contains("\n      - cli\n") {
        found.push("the publish job does not require the CLI job".to_owned());
    }
    if !push.contains("if: steps.reference.outputs.publish == 'true'") {
        found.push(format!("{PUSH:?} is not limited to a publish run"));
    }
    if !push.contains("docker push \"$IMAGE:$sha_tag\"") {
        found.push(format!("{PUSH:?} does not push the commit sha tag"));
    }
    found
}

#[test]
fn the_release_workflow_passes() {
    assert_eq!(problems(&workflow()), Vec::<String>::new());
}

#[test]
fn the_old_concurrency_key_is_reported() {
    let seeded = workflow().replace(
        "${{ inputs.release_tag || github.ref_name }}",
        "${{ inputs.release_tag || github.ref }}",
    );
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains("concurrency group"))
    );
}

#[test]
fn a_push_during_the_build_is_reported() {
    let text = workflow();
    let seeded = text.replace(
        ".\n          else\n            docker build --platform linux/amd64 --file Dockerfile.release --tag \"$REF\" .",
        ".\n            docker push \"$REF\"\n          else\n            docker build --platform linux/amd64 --file Dockerfile.release --tag \"$REF\" .",
    );
    assert_ne!(seeded, text);
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains("before the checks pass"))
    );
}

#[test]
fn a_missing_check_is_reported() {
    let seeded = workflow().replace(&format!("- name: {NO_PYTHON}"), "- name: Skipped check");
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains(NO_PYTHON))
    );
}

#[test]
fn a_missing_composed_step_is_reported() {
    let seeded = workflow().replace(&format!("- name: {COMPOSED}"), "- name: Skipped composed");
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains(COMPOSED))
    );
}

#[test]
fn a_missing_quality_job_is_reported() {
    let seeded = workflow().replace("\n  quality:\n", "\n  skipped-quality:\n");
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains("quality job is missing"))
    );
}

#[test]
fn a_missing_release_binary_download_is_reported() {
    let seeded = workflow().replace(
        "          gh run download \"${{ needs.gate.outputs.ci_run_id }}\"",
        "          echo skipped",
    );
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains("exact-SHA release binaries"))
    );
}

#[test]
fn a_missing_cli_job_is_reported() {
    let seeded = workflow().replace("\n  cli:\n", "\n  skipped-cli:\n");
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains("CLI release job is missing"))
    );
}

#[test]
fn a_desktop_job_is_reported() {
    let seeded = workflow().replace(
        "\n  publish:\n",
        "\n  desktop:\n    runs-on: ubuntu-latest\n\n  publish:\n",
    );
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains("desktop job must be removed"))
    );
}

#[test]
fn a_push_before_the_contract_check_is_reported() {
    let seeded = workflow().replace(
        "      - name: Check the image contract",
        "      - name: Push the verified image\n\
         \x20       if: steps.reference.outputs.publish == 'true'\n\
         \x20       run: |\n\
         \x20         docker push \"$REF\"\n\
         \x20         docker push \"$IMAGE:$sha_tag\"\n\n\
         \x20     - name: Check the image contract",
    );
    assert!(
        problems(&seeded)
            .iter()
            .any(|problem| problem.contains("runs after the image is published"))
    );
}
