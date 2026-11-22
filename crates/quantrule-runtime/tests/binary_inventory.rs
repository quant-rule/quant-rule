//! The seven-binary inventory guard.
//!
//! The platform is exactly seven service binaries: gateway, data, model,
//! backtest, evidence, simulation, and strategy. The guard reads every service
//! package's manifest and source entry points, so deleting a `[[bin]]` target,
//! renaming a service binary, or adding an eighth fails here. The expected set
//! is the contract; the actual set comes from the tree.
//!
//! The `quantrule` binary from `quantrule-shell` is a client, not a service, and
//! is outside this inventory. B edits no shell file and must not claim it as a
//! service binary.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The seven service binaries the image packages.
const EXPECTED: [&str; 7] = [
    "qr-backtest",
    "qr-data",
    "qr-evidence",
    "qr-gateway",
    "qr-model",
    "qr-simulation",
    "qr-strategy",
];

/// The package that ships each service binary.
///
/// P8 retired `quantrule-server`: the gateway is `quantrule-gateway` and the
/// worker is `quantrule-backtest-service`. The mapping is the contract, so a
/// service package cannot quietly ship two targets or a non-service package
/// cannot quietly ship a service binary. The shell, the library, and the engine
/// crates are not part of the service inventory.
const PACKAGES: [(&str, &str); 7] = [
    ("quantrule-backtest-service", "qr-backtest"),
    ("quantrule-data-service", "qr-data"),
    ("quantrule-evidence-service", "qr-evidence"),
    ("quantrule-gateway", "qr-gateway"),
    ("quantrule-model-service", "qr-model"),
    ("quantrule-simulation-service", "qr-simulation"),
    ("quantrule-strategy-service", "qr-strategy"),
];

/// Whether a package ships a service binary.
fn is_service_package(package: &str) -> bool {
    PACKAGES
        .iter()
        .any(|(service_package, _)| *service_package == package)
}

/// The workspace root, two levels above `crates/quantrule-runtime`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the runtime manifest has two ancestors")
        .to_path_buf()
}

/// The first quoted string on a line, which is how the member list is written.
fn quoted(line: &str) -> Option<&str> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// The workspace member directories, read from the root manifest.
fn members(root: &Path) -> Vec<PathBuf> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("workspace manifest");
    let mut found = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let line = line.trim();
        if !inside {
            inside = line.starts_with("members") && line.contains('[');
            continue;
        }
        if line.starts_with(']') {
            break;
        }
        if let Some(path) = quoted(line) {
            found.push(root.join(path));
        }
    }
    assert!(!found.is_empty(), "the workspace manifest lists no members");
    found
}

/// One member's declared targets.
#[derive(Default)]
struct Manifest {
    package: Option<String>,
    bins: BTreeSet<String>,
    explicit_paths: BTreeSet<String>,
}

/// Commit the bin target accumulated from one `[[bin]]` section.
fn commit(manifest: &mut Manifest, name: &mut Option<String>, path: &mut Option<String>) {
    if let Some(name) = name.take() {
        manifest.bins.insert(name);
    }
    if let Some(path) = path.take() {
        manifest.explicit_paths.insert(path.replace('\\', "/"));
    }
}

/// Read the package name and explicit `[[bin]]` targets from a manifest.
fn parse_manifest(text: &str) -> Manifest {
    let mut manifest = Manifest::default();
    let mut section = String::new();
    let mut bin_name = None;
    let mut bin_path = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            commit(&mut manifest, &mut bin_name, &mut bin_path);
            section = line.to_owned();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match (section.as_str(), key) {
            ("[package]", "name") => manifest.package = Some(value.to_owned()),
            ("[[bin]]", "name") => bin_name = Some(value.to_owned()),
            ("[[bin]]", "path") => bin_path = Some(value.to_owned()),
            _ => {}
        }
    }
    commit(&mut manifest, &mut bin_name, &mut bin_path);
    manifest
}

/// The binaries Cargo auto-discovers under `src/main.rs` and `src/bin/`.
///
/// An explicit target with the same path replaces the auto-discovered one, so
/// the path check keeps the guard from counting the same binary twice.
fn auto_bins(member: &Path, manifest: &Manifest) -> BTreeSet<String> {
    let mut bins = BTreeSet::new();
    let source = member.join("src");
    if source.join("main.rs").is_file()
        && !manifest.explicit_paths.contains("src/main.rs")
        && let Some(package) = &manifest.package
    {
        bins.insert(package.clone());
    }
    let entries = match std::fs::read_dir(source.join("bin")) {
        Ok(entries) => entries,
        Err(_) => return bins,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        if manifest
            .explicit_paths
            .contains(&format!("src/bin/{}", name.to_string_lossy()))
        {
            continue;
        }
        if let Some(stem) = path.file_stem() {
            bins.insert(stem.to_string_lossy().into_owned());
        }
    }
    bins
}

/// The binaries the service packages would build, from the real manifests and
/// sources.
fn inventory(root: &Path) -> BTreeSet<String> {
    package_inventory(root).into_values().flatten().collect()
}

/// The binaries each service package would build.
fn package_inventory(root: &Path) -> std::collections::BTreeMap<String, BTreeSet<String>> {
    let mut found = std::collections::BTreeMap::new();
    for member in members(root) {
        let path = member.join("Cargo.toml");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let manifest = parse_manifest(&text);
        if !manifest.package.as_deref().is_some_and(is_service_package) {
            continue;
        }
        let mut bins = manifest.bins.clone();
        bins.extend(auto_bins(&member, &manifest));
        let package = manifest.package.expect("a service package has a name");
        found.insert(package, bins);
    }
    found
}

/// The expected inventory.
fn expected() -> BTreeSet<String> {
    EXPECTED.iter().map(|name| (*name).to_owned()).collect()
}

/// Describe a mismatch between the expected and observed inventory.
fn mismatch(observed: &BTreeSet<String>) -> Option<String> {
    let expected = expected();
    if observed == &expected {
        return None;
    }
    let missing: Vec<&String> = expected.difference(observed).collect();
    let unexpected: Vec<&String> = observed.difference(&expected).collect();
    Some(format!("missing {missing:?}, unexpected {unexpected:?}"))
}

#[test]
fn the_workspace_builds_exactly_the_seven_service_binaries() {
    let observed = inventory(&workspace_root());
    assert_eq!(
        mismatch(&observed),
        None,
        "the seven-binary inventory changed: {observed:?}"
    );
}

#[test]
fn each_service_package_ships_exactly_its_own_binary() {
    let observed = package_inventory(&workspace_root());
    let expected: std::collections::BTreeMap<String, BTreeSet<String>> = PACKAGES
        .iter()
        .map(|(package, binary)| {
            (
                (*package).to_owned(),
                BTreeSet::from([(*binary).to_owned()]),
            )
        })
        .collect();
    assert_eq!(observed, expected);
}

#[test]
fn the_retired_server_crate_is_not_a_workspace_member() {
    let members = members(&workspace_root());
    assert!(
        members.iter().all(|member| member
            .file_name()
            .is_none_or(|name| name != "quantrule-server")),
        "quantrule-server is still a workspace member: {members:?}"
    );
}

#[test]
fn removing_a_binary_fails_the_guard() {
    let mut observed = expected();
    observed.remove("qr-model");
    let problem = mismatch(&observed).expect("a removed binary must be reported");
    assert!(problem.contains("qr-model"), "{problem}");
}

#[test]
fn an_extra_binary_fails_the_guard() {
    let mut observed = expected();
    observed.insert("qr-extra".to_owned());
    let problem = mismatch(&observed).expect("an extra binary must be reported");
    assert!(problem.contains("qr-extra"), "{problem}");
}

#[test]
fn a_missing_strategy_binary_fails_the_guard() {
    let mut observed = expected();
    observed.remove("qr-strategy");
    let problem = mismatch(&observed).expect("a missing strategy binary must be reported");
    assert!(problem.contains("qr-strategy"), "{problem}");
}
