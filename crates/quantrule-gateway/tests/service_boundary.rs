//! The #651 service boundary guard.
//!
//! The architecture is proven, not asserted. This test reads the workspace
//! manifests, the service sources, and the SQL migrations, and fails when:
//!
//! - the gateway reaches `quantrule-app`, an engine, or a domain service crate,
//!   directly or through an intermediate dependency;
//! - a service package ships anything other than its one binary, or the six
//!   service names change;
//! - a service names a schema it does not own;
//! - the simulation service reads a strategy, writes a run, publishes a queue
//!   message, or computes a model.
//!
//! Each rule has a seeded-violation case, so a guard that stops matching is a
//! test failure instead of a silent pass. The concrete `cargo tree -p
//! quantrule-gateway` output is the release-time confirmation; this test is the
//! fast tier's compile/source proof.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One service package, its one binary, and the schemas it may name.
struct Service {
    package: &'static str,
    binary: &'static str,
    schemas: &'static [&'static str],
    imports: &'static [&'static str],
}

/// The six service packages and the one binary each ships.
const SERVICES: [Service; 6] = [
    Service {
        package: "quantrule-gateway",
        binary: "qr-gateway",
        schemas: &["gateway"],
        imports: &[
            "quantrule_api",
            "quantrule_gateway",
            "quantrule_runtime",
            "quantrule_service_api",
        ],
    },
    Service {
        package: "quantrule-data-service",
        binary: "qr-data",
        schemas: &["catalog"],
        imports: &[
            "quantrule_api",
            "quantrule_app",
            "quantrule_data",
            "quantrule_data_service",
            "quantrule_runtime",
            "quantrule_service_api",
        ],
    },
    Service {
        package: "quantrule-model-service",
        binary: "qr-model",
        schemas: &[],
        imports: &[
            "quantrule_api",
            "quantrule_app",
            "quantrule_data",
            "quantrule_model",
            "quantrule_model_service",
            "quantrule_runtime",
            "quantrule_service_api",
        ],
    },
    Service {
        package: "quantrule-backtest-service",
        binary: "qr-backtest",
        schemas: &[],
        imports: &[
            "quantrule_api",
            "quantrule_app",
            "quantrule_backtest_service",
            "quantrule_data",
            "quantrule_runtime",
            "quantrule_service_api",
        ],
    },
    Service {
        package: "quantrule-evidence-service",
        binary: "qr-evidence",
        schemas: &["runs", "research", "stage"],
        imports: &[
            "quantrule_api",
            "quantrule_evidence_service",
            "quantrule_research",
            "quantrule_runtime",
            "quantrule_service_api",
        ],
    },
    Service {
        package: "quantrule-simulation-service",
        binary: "qr-simulation",
        schemas: &["simulation"],
        imports: &[
            "quantrule_api",
            "quantrule_runtime",
            "quantrule_simulation",
            "quantrule_simulation_service",
        ],
    },
];

/// Every schema a service owns in v1.0.0.
const SCHEMAS: [&str; 6] = [
    "catalog",
    "runs",
    "stage",
    "research",
    "simulation",
    "gateway",
];

/// The packages the gateway must never reach, directly or transitively.
const GATEWAY_FORBIDDEN: [&str; 9] = [
    "quantrule-app",
    "quantrule-data",
    "quantrule-model",
    "quantrule-backtest",
    "quantrule-data-service",
    "quantrule-model-service",
    "quantrule-backtest-service",
    "quantrule-evidence-service",
    "quantrule-simulation-service",
];

/// The simulation service's forbidden imports, writes, and model calls.
const SIMULATION_FORBIDDEN: [&str; 30] = [
    "quantrule_app",
    "quantrule_data",
    "quantrule_model",
    "quantrule_backtest",
    "AppService",
    "ExecutionEngine",
    "PortfolioLedger",
    "RunStore",
    "RunManifest",
    "canonical_digest",
    "Queue",
    "QueueError",
    "ROUTING_",
    "strategy::",
    "execution::",
    "portfolio::",
    "run_backtest",
    "run.requested",
    "run.computed",
    "run.failed",
    "INSERT INTO runs.",
    "UPDATE runs.",
    "DELETE FROM runs.",
    "SELECT * FROM runs.",
    "FROM runs.runs",
    "INSERT INTO runs",
    "UPDATE runs",
    "DELETE FROM runs",
    "FROM runs",
    "JOIN runs",
];

/// One workspace member's manifest, reduced to what the guard reads.
#[derive(Default, Clone)]
struct Manifest {
    package: Option<String>,
    dependencies: BTreeSet<String>,
    dev_dependencies: BTreeSet<String>,
    bins: BTreeSet<String>,
}

impl Manifest {
    fn all_dependencies(&self) -> BTreeSet<String> {
        self.dependencies
            .union(&self.dev_dependencies)
            .cloned()
            .collect()
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the gateway crate lives under crates/")
        .to_path_buf()
}

fn quoted(line: &str) -> Option<&str> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

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

fn parse_manifest(text: &str) -> Manifest {
    let mut manifest = Manifest::default();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            section = line.to_owned();
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = line
            .split_once('=')
            .map(|(_, value)| value.trim().trim_matches('"'))
            .unwrap_or_default();
        match section.as_str() {
            "[package]" if key == "name" => manifest.package = Some(value.to_owned()),
            "[dependencies]" => {
                if key.starts_with("quantrule-") {
                    manifest.dependencies.insert(key.to_owned());
                }
            }
            "[dev-dependencies]" => {
                if key.starts_with("quantrule-") {
                    manifest.dev_dependencies.insert(key.to_owned());
                }
            }
            "[[bin]]" if key == "name" => {
                manifest.bins.insert(value.to_owned());
            }
            _ => {}
        }
    }
    manifest
}

/// Every workspace manifest keyed by package name.
fn manifests(root: &Path) -> BTreeMap<String, Manifest> {
    let mut found = BTreeMap::new();
    for member in members(root) {
        let path = member.join("Cargo.toml");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let manifest = parse_manifest(&text);
        if let Some(package) = &manifest.package {
            found.insert(package.clone(), manifest);
        }
    }
    found
}

fn closure(graph: &BTreeMap<String, BTreeSet<String>>, root: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![root.to_owned()];
    while let Some(name) = pending.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(dependencies) = graph.get(&name) {
            pending.extend(dependencies.iter().cloned());
        }
    }
    seen
}

fn forbidden_links(linked: &BTreeSet<String>, forbidden: &[&str]) -> Vec<String> {
    forbidden
        .iter()
        .filter(|name| linked.contains(**name))
        .map(|name| (*name).to_owned())
        .collect()
}

fn rm_files(root: &Path, extension: &str, found: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rm_files(&path, extension, found);
        } else if path.extension().is_some_and(|value| value == extension) {
            found.push(path);
        }
    }
}

/// The code of one Rust source, without `#[cfg(test)]` and without comments.
fn code_only(text: &str) -> String {
    let text = text.split("#[cfg(test)]").next().unwrap_or(text);
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(character) = chars.next() {
        if in_string {
            out.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if character == '"' {
            in_string = true;
            out.push(character);
            continue;
        }
        if character == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if character == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut previous = '\0';
            for next in chars.by_ref() {
                if previous == '*' && next == '/' {
                    break;
                }
                previous = next;
            }
            continue;
        }
        out.push(character);
    }
    out
}

/// The string literals of one Rust source, with the line they start on.
fn literals(text: &str) -> Vec<(usize, String)> {
    let text = text.split("#[cfg(test)]").next().unwrap_or(text);
    let mut found = Vec::new();
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut line = 1;
    let mut start = 1;
    let mut current = String::new();
    while let Some(character) = chars.next() {
        if character == '\n' {
            line += 1;
        }
        if in_string {
            if escaped {
                escaped = false;
                if character != '\n' {
                    current.push(character);
                }
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                found.push((start, std::mem::take(&mut current)));
                in_string = false;
            } else {
                current.push(character);
            }
            continue;
        }
        if character == '"' {
            in_string = true;
            start = line;
        } else if character == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    line += 1;
                    break;
                }
            }
        } else if character == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut previous = '\0';
            for next in chars.by_ref() {
                if next == '\n' {
                    line += 1;
                }
                if previous == '*' && next == '/' {
                    break;
                }
                previous = next;
            }
        }
    }
    found
}

fn schema_hits(text: &str, allowed: &[&str]) -> Vec<String> {
    let mut hits = Vec::new();
    for (line, literal) in literals(text) {
        for schema in SCHEMAS {
            if allowed.contains(&schema) {
                continue;
            }
            if literal.contains(&format!("{schema}.")) {
                hits.push(format!("line {line}: {schema}."));
            }
        }
    }
    hits
}

fn sql_schema_hits(text: &str, allowed: &[&str]) -> Vec<String> {
    let mut hits = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = line.split("--").next().unwrap_or_default();
        for schema in SCHEMAS {
            if allowed.contains(&schema) {
                continue;
            }
            if code.contains(&format!("{schema}.")) {
                hits.push(format!("line {}: {schema}.", index + 1));
            }
        }
    }
    hits
}

/// The schema-qualified names one service names in its sources and migrations.
fn foreign_schema_reads(root: &Path, service: &Service) -> Vec<String> {
    let package = root.join("crates").join(service.package);
    let mut violations = Vec::new();
    let mut files = Vec::new();
    rm_files(&package.join("src"), "rs", &mut files);
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read a service source");
        for hit in schema_hits(&text, service.schemas) {
            violations.push(format!("{}: {hit}", file.display()));
        }
    }
    let mut sql = Vec::new();
    rm_files(&package.join("migrations"), "sql", &mut sql);
    for file in sql {
        let text = std::fs::read_to_string(&file).expect("read a migration");
        for hit in sql_schema_hits(&text, service.schemas) {
            violations.push(format!("{}: {hit}", file.display()));
        }
    }
    violations
}

/// The forbidden identifiers one service names in production source.
fn forbidden_source_reads(root: &Path, package: &str, forbidden: &[&str]) -> Vec<String> {
    let mut violations = Vec::new();
    let mut files = Vec::new();
    rm_files(
        &root.join("crates").join(package).join("src"),
        "rs",
        &mut files,
    );
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read a service source");
        let code = code_only(&text);
        for (index, line) in code.lines().enumerate() {
            for name in forbidden {
                if line.contains(name) {
                    violations.push(format!("{}:{}: {name}", file.display(), index + 1));
                }
            }
        }
    }
    violations
}

/// Every workspace crate one service names in its production source.
fn crate_imports(root: &Path, package: &str) -> BTreeSet<String> {
    let mut files = Vec::new();
    rm_files(
        &root.join("crates").join(package).join("src"),
        "rs",
        &mut files,
    );
    let mut found = BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read a service source");
        let code = code_only(&text);
        let mut rest = code.as_str();
        while let Some(position) = rest.find("quantrule_") {
            rest = &rest[position..];
            let end = rest
                .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
                .unwrap_or(rest.len());
            found.insert(rest[..end].to_owned());
            rest = &rest[end..];
        }
    }
    found
}

fn import_problems(
    observed: &BTreeMap<String, BTreeSet<String>>,
    services: &[Service],
) -> Vec<String> {
    let mut problems = Vec::new();
    for service in services {
        let Some(imports) = observed.get(service.package) else {
            problems.push(format!("{} has no observed imports", service.package));
            continue;
        };
        for name in imports {
            if !service.imports.contains(&name.as_str()) {
                problems.push(format!("{} imports {name}", service.package));
            }
        }
    }
    problems
}

fn binary_problems(observed: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let mut problems = Vec::new();
    for service in &SERVICES {
        match observed.get(service.package) {
            Some(bins) if bins == &BTreeSet::from([service.binary.to_owned()]) => {}
            Some(bins) => problems.push(format!(
                "{} ships {bins:?}; expected exactly {:?}",
                service.package, service.binary
            )),
            None => problems.push(format!(
                "{} is not a package with a binary",
                service.package
            )),
        }
    }
    for package in observed.keys() {
        if !SERVICES.iter().any(|service| service.package == package) {
            problems.push(format!("{package} is not a service package"));
        }
    }
    problems
}

fn observed_binaries(root: &Path) -> BTreeMap<String, BTreeSet<String>> {
    manifests(root)
        .into_iter()
        .filter(|(package, _)| SERVICES.iter().any(|service| service.package == package))
        .map(|(package, manifest)| (package, manifest.bins))
        .collect()
}

#[test]
fn the_gateway_links_no_app_engine_or_service_crate() {
    let root = workspace_root();
    let all = manifests(&root);
    let graph: BTreeMap<String, BTreeSet<String>> = all
        .iter()
        .map(|(package, manifest)| (package.clone(), manifest.dependencies.clone()))
        .collect();
    let linked = closure(&graph, "quantrule-gateway");
    assert_eq!(
        forbidden_links(&linked, &GATEWAY_FORBIDDEN),
        Vec::<String>::new(),
        "the gateway dependency closure reaches a forbidden crate"
    );

    let gateway = all.get("quantrule-gateway").expect("the gateway package");
    assert_eq!(
        forbidden_links(&gateway.all_dependencies(), &GATEWAY_FORBIDDEN),
        Vec::<String>::new(),
        "the gateway manifest names a forbidden crate, including as a dev-dependency"
    );
}

#[test]
fn the_gateway_source_links_no_app_engine_or_service_crate() {
    let violations = forbidden_source_reads(
        &workspace_root(),
        "quantrule-gateway",
        &[
            "quantrule_app",
            "quantrule_data",
            "quantrule_model",
            "quantrule_backtest",
            "quantrule-data-service",
            "quantrule-model-service",
            "quantrule-backtest-service",
            "quantrule-evidence-service",
            "quantrule-simulation-service",
        ],
    );
    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn every_service_ships_exactly_one_binary() {
    let problems = binary_problems(&observed_binaries(&workspace_root()));
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn no_service_depends_on_another_service_crate() {
    let root = workspace_root();
    let all = manifests(&root);
    let services: BTreeSet<String> = SERVICES
        .iter()
        .map(|service| service.package.to_owned())
        .collect();
    let mut violations = Vec::new();
    for service in &SERVICES {
        let manifest = all.get(service.package).expect("the service package");
        for dependency in manifest.all_dependencies() {
            if services.contains(&dependency) && dependency != service.package {
                violations.push(format!("{} depends on {dependency}", service.package));
            }
        }
    }
    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn each_service_imports_only_its_allowed_crates() {
    let root = workspace_root();
    let observed: BTreeMap<String, BTreeSet<String>> = SERVICES
        .iter()
        .map(|service| {
            (
                service.package.to_owned(),
                crate_imports(&root, service.package),
            )
        })
        .collect();
    let problems = import_problems(&observed, &SERVICES);
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_seeded_disallowed_import_fails_the_allowlist() {
    let observed = BTreeMap::from([(
        "quantrule-simulation-service".to_owned(),
        BTreeSet::from(["quantrule_api".to_owned(), "quantrule_backtest".to_owned()]),
    )]);
    let services = &SERVICES[5..6];
    let problems = import_problems(&observed, services);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("quantrule_backtest")),
        "{problems:#?}"
    );
}

#[test]
fn each_service_names_only_its_own_schema() {
    let root = workspace_root();
    let mut violations = Vec::new();
    for service in &SERVICES {
        violations.extend(foreign_schema_reads(&root, service));
    }
    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn simulation_is_isolated_from_runs_strategy_and_the_queue() {
    let root = workspace_root();
    let all = manifests(&root);
    let graph: BTreeMap<String, BTreeSet<String>> = all
        .iter()
        .map(|(package, manifest)| (package.clone(), manifest.dependencies.clone()))
        .collect();
    let linked = closure(&graph, "quantrule-simulation-service");
    for forbidden in [
        "quantrule-app",
        "quantrule-data",
        "quantrule-model",
        "quantrule-backtest",
        "quantrule-data-service",
        "quantrule-model-service",
        "quantrule-backtest-service",
        "quantrule-evidence-service",
        "quantrule-gateway",
    ] {
        assert!(
            !linked.contains(forbidden),
            "the simulation closure reaches {forbidden}"
        );
    }

    let violations =
        forbidden_source_reads(&root, "quantrule-simulation-service", &SIMULATION_FORBIDDEN);
    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn a_seeded_gateway_engine_edge_fails_the_closure_guard() {
    let graph = BTreeMap::from([
        (
            "quantrule-gateway".to_owned(),
            BTreeSet::from(["quantrule-runtime".to_owned()]),
        ),
        (
            "quantrule-runtime".to_owned(),
            BTreeSet::from(["quantrule-app".to_owned()]),
        ),
    ]);
    let linked = closure(&graph, "quantrule-gateway");
    let hits = forbidden_links(&linked, &GATEWAY_FORBIDDEN);
    assert_eq!(hits, vec!["quantrule-app".to_owned()]);
}

#[test]
fn a_seeded_foreign_schema_read_fails_the_schema_guard() {
    let seeded = "pub async fn read() {\n    let sql = \"SELECT * FROM runs.runs\";\n}\n";
    let hits = schema_hits(seeded, &["catalog"]);
    assert!(hits.iter().any(|hit| hit.contains("runs.")), "{hits:?}");
}

#[test]
fn a_seeded_strategy_read_fails_the_simulation_guard() {
    let root = std::env::temp_dir().join(format!(
        "quantrule-simulation-seeded-{}",
        std::process::id()
    ));
    let source = root.join("crates/quantrule-simulation-service/src");
    std::fs::create_dir_all(&source).expect("create the seeded source tree");
    std::fs::write(
        source.join("read.rs"),
        "use quantrule_backtest::strategy::Rule;\n\npub fn read() -> Rule { Rule::default() }\n",
    )
    .expect("write the seeded source");
    let violations =
        forbidden_source_reads(&root, "quantrule-simulation-service", &SIMULATION_FORBIDDEN);
    std::fs::remove_dir_all(&root).expect("remove the seeded source tree");
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("quantrule_backtest")),
        "{violations:#?}"
    );
}

#[test]
fn a_seeded_second_binary_and_a_retired_package_fail_the_inventory_guard() {
    let mut observed = observed_binaries(&workspace_root());
    observed
        .get_mut("quantrule-gateway")
        .expect("the gateway package")
        .insert("qr-extra".to_owned());
    observed.insert(
        "quantrule-retired-service".to_owned(),
        BTreeSet::from(["qr-retired".to_owned()]),
    );
    let problems = binary_problems(&observed);
    assert!(
        problems.iter().any(|problem| problem.contains("qr-extra")),
        "{problems:#?}"
    );
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("quantrule-retired-service")),
        "{problems:#?}"
    );
}

#[test]
fn the_guard_reads_production_code_not_its_own_fixtures() {
    let seeded = "//! The gateway must not link `quantrule-app`.\n// use quantrule_app::Service;\n";
    let code = code_only(seeded);
    assert!(!code.contains("quantrule-app"));
    assert!(!code.contains("quantrule_app"));
    assert!(code.is_empty() || code.trim().is_empty());
}
