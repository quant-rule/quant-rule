//! Parity guard: the shell command and the shared use case write the same model
//! bundle from the same fixture. The comparison covers file names and bytes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config-run/run.json")
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parity")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    path
}

fn bundle(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    fn collect(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(path).expect("the bundle is readable") {
            let entry = entry.expect("the bundle entry is readable");
            let current = entry.path();
            if current.is_dir() {
                collect(root, &current, files);
            } else {
                let relative = current
                    .strip_prefix(root)
                    .expect("the artifact is under the bundle root")
                    .to_string_lossy()
                    .into_owned();
                files.insert(
                    relative,
                    fs::read(&current).expect("the artifact is readable"),
                );
            }
        }
    }
    collect(root, root, &mut files);
    files
}

#[test]
fn the_shell_and_the_app_write_the_same_model_bundle() {
    let shell_out = output_dir("shell");
    let app_out = output_dir("app");
    let config = fixture();

    let status = Command::new(BIN)
        .args([
            "report",
            "model",
            "--config",
            config.to_str().expect("the fixture path is UTF-8"),
            "--out",
            shell_out.to_str().expect("the output path is UTF-8"),
        ])
        .status()
        .expect("the shell command runs");
    assert!(status.success(), "the shell report command succeeds");

    let request = quantrule_app::stages::model::ModelReportRequest {
        config: &config,
        spec: None,
        out: Some(&app_out),
    };
    quantrule_app::service::AppService::run_model_report(&request)
        .expect("the shared model report succeeds");

    assert_eq!(bundle(&shell_out), bundle(&app_out));
}
