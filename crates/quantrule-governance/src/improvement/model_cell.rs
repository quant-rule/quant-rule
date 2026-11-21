//! Run one pre-registered model cell and merge its yearly bundles.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use super::{model_condition, model_path, model_prediction, store};
use crate::research::merge;

const INSTRUMENTS: [&str; 4] = ["XAU", "BTC", "SP500", "EURUSD"];

struct Args {
    cell_dir: String,
    phase: String,
    out: String,
    runs_dir: Option<String>,
    model: Option<String>,
    qr: Option<String>,
    timeframe: Option<String>,
    selection: Option<String>,
    state_file: Option<String>,
    condition_file: Option<String>,
    path_menu: Option<String>,
    instruments: Option<String>,
    count_only: bool,
    horizon: i64,
    target_atr: f64,
    stop_atr: f64,
    cost_bps: f64,
    primary_cells: usize,
    seed: u64,
    family: String,
    repo_root: String,
}

impl Args {
    fn partition(&self) -> (&'static str, &'static str, &'static str, &'static [i64]) {
        match self.phase.as_str() {
            "validation" => (
                "model_validation",
                "2021-01-01",
                "2022-12-31",
                &[2021, 2022],
            ),
            _ => ("model_dev", "2018-01-01", "2020-12-31", &[2018, 2019, 2020]),
        }
    }
}

/// Run the model-cell orchestrator.
///
/// # Errors
///
/// Returns a message for a missing cell input, a failed shell command, or a
/// write failure.
pub fn run(raw: &[String]) -> Result<(), String> {
    let args = parse(raw)?;
    let repo_root = Path::new(".")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."));
    let cell_dir = absolute(Path::new(&args.cell_dir), &repo_root);
    let out = absolute(Path::new(&args.out), &repo_root);
    fs::create_dir_all(&out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let runs_dir = args
        .runs_dir
        .as_ref()
        .map(|value| absolute(Path::new(value), &repo_root))
        .unwrap_or_else(|| {
            cell_dir
                .parent()
                .and_then(Path::parent)
                .unwrap_or(&repo_root)
                .join("runs")
        });
    let model_path = if let Some(value) = &args.model {
        let path = absolute(Path::new(value), &repo_root);
        if !path.is_file() {
            return Err(format!("model config not found: {}", path.display()));
        }
        Some(path)
    } else {
        let path = cell_dir.join("model.json");
        path.is_file().then_some(path)
    };
    let (partition, split_start, split_end, years) = args.partition();
    let qr = shell_binary(&repo_root, args.qr.as_deref())?;
    let cell_id = cell_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let timeframe = resolve_timeframe(
        args.timeframe.as_deref(),
        &cell_dir,
        args.selection.as_deref(),
    )?;
    if args.count_only && timeframe.is_none() {
        return Err("--count-only requires --timeframe".to_owned());
    }
    let instruments = cell_instruments(&cell_dir, args.instruments.as_deref())?;
    let state_path = args
        .state_file
        .as_ref()
        .map(|value| absolute(Path::new(value), &repo_root));
    let condition_path = args
        .condition_file
        .as_ref()
        .map(|value| absolute(Path::new(value), &repo_root));
    if let Some(path) = &condition_path
        && !path.is_file()
    {
        return Err(format!("condition file not found: {}", path.display()));
    }
    if state_path.is_some() && condition_path.is_some() {
        return Err("--state-file and --condition-file are mutually exclusive".to_owned());
    }
    let path_menu = args
        .path_menu
        .as_ref()
        .map(|value| absolute(Path::new(value), &repo_root));
    let gate = state_path
        .as_ref()
        .map(|path| model_prediction::load_state_gate(&path.display().to_string()))
        .transpose()?;
    let mut bundles: Vec<(String, PathBuf)> = Vec::new();
    let mut counts: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut generated_gate: Option<PathBuf> = None;
    for instrument in &instruments {
        let spec = cell_spec(&cell_dir, instrument, timeframe);
        if !spec.is_file() {
            return Err(format!("spec not found: {}", spec.display()));
        }
        let mut yearly = Vec::new();
        for year in years {
            let mut run = runs_dir
                .join(instrument)
                .join(year.to_string())
                .join("run.json");
            if !run.is_file() {
                return Err(format!("run config not found: {}", run.display()));
            }
            let year_out = out.join(instrument).join(year.to_string());
            if let Some(model) = &model_path {
                run = prepare_run_config(&run, Some(model), &year_out.join("config"))?;
            }
            let status = Command::new(&qr)
                .current_dir(&repo_root)
                .args([
                    "report",
                    "model",
                    "--config",
                    &run.display().to_string(),
                    "--spec",
                    &spec.display().to_string(),
                    "--out",
                    &year_out.display().to_string(),
                ])
                .output()
                .map_err(|error| {
                    format!("cannot start {cell_id} {instrument} {year} model: {error}")
                })?;
            if !status.status.success() {
                let message = String::from_utf8_lossy(&status.stderr).trim().to_owned();
                return Err(format!(
                    "{cell_id} {instrument} {year} model failed: {message}"
                ));
            }
            let bundle = year_out.join("model");
            if !bundle.join("prediction.csv").is_file() {
                return Err(format!(
                    "prediction.csv not found under {}",
                    bundle.display()
                ));
            }
            yearly.push((*year, bundle));
        }
        if args.count_only && condition_path.is_none() {
            let values: Vec<BTreeMap<String, usize>> = yearly
                .iter()
                .map(|(_, bundle)| {
                    count_bundle(bundle, timeframe.unwrap_or(0), gate.as_ref(), args.horizon)
                })
                .collect::<Result<_, _>>()?;
            let mut entry = BTreeMap::new();
            entry.insert("n".to_owned(), values.iter().map(|value| value["n"]).sum());
            entry.insert(
                "clusters".to_owned(),
                values.iter().map(|value| value["clusters"]).sum(),
            );
            counts.insert(instrument.clone(), entry);
            continue;
        }
        let merged = out.join(instrument).join("model");
        merge::merge_bundles(&merged, &yearly)?;
        let mut bundle = merged;
        if let Some(condition) = &condition_path {
            let conditioned = out.join(instrument).join("conditioned");
            let condition_args = vec![
                "--bundle".to_owned(),
                bundle.display().to_string(),
                "--condition".to_owned(),
                condition.display().to_string(),
                "--registry".to_owned(),
                repo_root
                    .join("docs/registry/indicators.json")
                    .display()
                    .to_string(),
                "--out".to_owned(),
                conditioned.display().to_string(),
            ];
            if condition_args.iter().all(|value| !value.is_empty()) {
                model_condition::run(&condition_args)?;
            }
            bundle = conditioned;
            generated_gate = Some(bundle.join("state.json"));
        }
        if args.count_only {
            let count_gate = if let Some(path) = &generated_gate {
                Some(model_prediction::load_state_gate(
                    &path.display().to_string(),
                )?)
            } else {
                gate.clone()
            };
            counts.insert(
                instrument.clone(),
                count_bundle(
                    &bundle,
                    timeframe.unwrap_or(0),
                    count_gate.as_ref(),
                    args.horizon,
                )?,
            );
            continue;
        }
        bundles.push((instrument.clone(), bundle));
    }
    if args.count_only {
        let pooled_n: usize = counts.values().map(|value| value["n"]).sum();
        let pooled_clusters: usize = counts.values().map(|value| value["clusters"]).sum();
        let document = json!({
            "schema": "quantrule.model-cell-count/v1", "cell_id": cell_id, "phase": args.phase,
            "timeframe_secs": timeframe,
            "state_file": generated_gate.as_ref().or(state_path.as_ref()).map(|path| path.display().to_string()),
            "condition_file": condition_path.as_ref().map(|path| path.display().to_string()),
            "model_config_sha256": model_path.as_ref().map(|path| file_digest(path)).transpose()?,
            "instruments": instruments, "years": years,
            "counts": counts, "pooled": {"n": pooled_n, "clusters": pooled_clusters}
        });
        store::write_json(&out.join("count.json"), &document)?;
        println!(
            "ok {cell_id} count timeframe={} n={} clusters={} -> {}",
            timeframe.unwrap_or(0),
            pooled_n,
            pooled_clusters,
            out.display()
        );
        return Ok(());
    }
    let digest = merge::prediction_digest(&bundles)?;
    let evaluation = out.join("evaluation");
    let mut prediction_args = vec![
        "--spec-id".to_owned(),
        cell_id.clone(),
        "--family".to_owned(),
        args.family.clone(),
        "--partition".to_owned(),
        partition.to_owned(),
        "--split-start".to_owned(),
        split_start.to_owned(),
        "--split-end".to_owned(),
        split_end.to_owned(),
        "--horizon".to_owned(),
        args.horizon.to_string(),
        "--target-atr".to_owned(),
        args.target_atr.to_string(),
        "--stop-atr".to_owned(),
        args.stop_atr.to_string(),
        "--cost-bps".to_owned(),
        args.cost_bps.to_string(),
        "--primary-cells".to_owned(),
        args.primary_cells.to_string(),
        "--seed".to_owned(),
        args.seed.to_string(),
        "--out".to_owned(),
        evaluation.display().to_string(),
    ];
    if let Some(timeframe) = timeframe {
        prediction_args.extend(["--timeframe".to_owned(), timeframe.to_string()]);
    }
    if let Some(path) = generated_gate.as_ref().or(state_path.as_ref()) {
        prediction_args.extend(["--state-file".to_owned(), path.display().to_string()]);
    }
    for (instrument, bundle) in &bundles {
        prediction_args.extend([
            "--bundle".to_owned(),
            format!("{instrument}={}", bundle.display()),
        ]);
    }
    model_prediction::run(&prediction_args)?;
    let mut document = store::read_json(&evaluation.join("model_prediction.json"))?;
    if let Some(model) = &model_path {
        document["model_config_sha256"] = json!(file_digest(model)?);
        document["model_config_path"] = json!(model.display().to_string());
    }
    if let Some(condition) = &condition_path {
        document["condition_file"] = json!(condition.display().to_string());
        document["condition_sha256"] = json!(file_digest(condition)?);
    }
    if let Some(menu) = &path_menu {
        let path_out = out.join("path");
        let mut path_args = vec![
            "--spec-id".to_owned(),
            cell_id.clone(),
            "--family".to_owned(),
            args.family.clone(),
            "--partition".to_owned(),
            partition.to_owned(),
            "--cost-bps".to_owned(),
            args.cost_bps.to_string(),
            "--menu".to_owned(),
            menu.display().to_string(),
            "--out".to_owned(),
            path_out.display().to_string(),
        ];
        if condition_path.is_some() {
            path_args.extend(["--condition-column".to_owned(), "condition".to_owned()]);
        }
        for (instrument, bundle) in &bundles {
            path_args.extend([
                "--bundle".to_owned(),
                format!("{instrument}={}", bundle.display()),
            ]);
        }
        model_path::run(&path_args)?;
        let path_document = store::read_json(&path_out.join("path.json"))?;
        document["path"] = json!({
            "path": path_out.join("path.json").display().to_string(),
            "menu": path_document.get("menu").cloned().unwrap_or(Value::Null),
            "summaries": path_document.get("summaries").cloned().unwrap_or(Value::Null),
        });
    }
    document["phase"] = json!(args.phase);
    document["prediction_digest"] = json!(digest);
    document["years"] = json!(years);
    store::write_json(&out.join("model_prediction.json"), &document)?;
    let source = evaluation.join("model_prediction.md");
    if source.is_file() {
        fs::copy(&source, out.join("model_prediction.md"))
            .map_err(|error| format!("cannot copy model_prediction.md: {error}"))?;
    }
    println!(
        "ok {cell_id} phase={} instruments={} years={} grade={} verdict={} digest={}",
        args.phase,
        bundles.len(),
        years.len(),
        document["grade"].as_str().unwrap_or(""),
        document["verdict"].as_str().unwrap_or(""),
        &digest[..12.min(digest.len())]
    );
    Ok(())
}

fn parse(raw: &[String]) -> Result<Args, String> {
    let mut args = Args {
        cell_dir: String::new(),
        phase: "dev".to_owned(),
        out: String::new(),
        runs_dir: None,
        model: None,
        qr: None,
        timeframe: None,
        selection: None,
        state_file: None,
        condition_file: None,
        path_menu: None,
        instruments: None,
        count_only: false,
        horizon: 20,
        target_atr: 2.0,
        stop_atr: 1.0,
        cost_bps: 6.0,
        primary_cells: 5,
        seed: 0,
        family: "M2".to_owned(),
        repo_root: ".".to_owned(),
    };
    let mut index = 0;
    while index < raw.len() {
        let name = raw[index].as_str();
        let next = |index: &mut usize| -> Result<String, String> {
            *index += 1;
            raw.get(*index)
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match name {
            "--cell-dir" => args.cell_dir = next(&mut index)?,
            "--phase" => args.phase = next(&mut index)?,
            "--out" => args.out = next(&mut index)?,
            "--runs-dir" => args.runs_dir = Some(next(&mut index)?),
            "--model" => args.model = Some(next(&mut index)?),
            "--qr" => args.qr = Some(next(&mut index)?),
            "--timeframe" => args.timeframe = Some(next(&mut index)?),
            "--selection" => args.selection = Some(next(&mut index)?),
            "--state-file" => args.state_file = Some(next(&mut index)?),
            "--condition-file" => args.condition_file = Some(next(&mut index)?),
            "--path-menu" => args.path_menu = Some(next(&mut index)?),
            "--instruments" => args.instruments = Some(next(&mut index)?),
            "--count-only" => args.count_only = true,
            "--horizon" => {
                args.horizon = next(&mut index)?
                    .parse()
                    .map_err(|_| "--horizon invalid".to_owned())?
            }
            "--target-atr" => {
                args.target_atr = next(&mut index)?
                    .parse()
                    .map_err(|_| "--target-atr invalid".to_owned())?
            }
            "--stop-atr" => {
                args.stop_atr = next(&mut index)?
                    .parse()
                    .map_err(|_| "--stop-atr invalid".to_owned())?
            }
            "--cost-bps" => {
                args.cost_bps = next(&mut index)?
                    .parse()
                    .map_err(|_| "--cost-bps invalid".to_owned())?
            }
            "--primary-cells" => {
                args.primary_cells = next(&mut index)?
                    .parse()
                    .map_err(|_| "--primary-cells invalid".to_owned())?
            }
            "--seed" => {
                args.seed = next(&mut index)?
                    .parse()
                    .map_err(|_| "--seed invalid".to_owned())?
            }
            "--family" => args.family = next(&mut index)?,
            "--repo-root" => args.repo_root = next(&mut index)?,
            "--python"
            | "--evaluator"
            | "--condition-evaluator"
            | "--path-evaluator"
            | "--timeout" => {
                let _ = next(&mut index)?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    if args.cell_dir.is_empty() || args.out.is_empty() {
        return Err("--cell-dir and --out are required".to_owned());
    }
    Ok(args)
}

fn shell_binary(root: &Path, requested: Option<&str>) -> Result<PathBuf, String> {
    if let Some(requested) = requested {
        let path = absolute(Path::new(requested), root);
        if !path.is_file() {
            return Err(format!("quantrule binary not found: {}", path.display()));
        }
        return Ok(path);
    }
    for candidate in ["target/release/quantrule", "target/debug/quantrule"] {
        let path = root.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err("no quantrule binary found; build one or pass --qr target/release/quantrule".to_owned())
}

fn cell_instruments(cell_dir: &Path, requested: Option<&str>) -> Result<Vec<String>, String> {
    if let Some(requested) = requested {
        let values: Vec<String> = requested
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect();
        if values.is_empty() {
            return Err("--instruments is empty".to_owned());
        }
        return Ok(values);
    }
    let coverage = cell_dir.join("coverage.json");
    if coverage.is_file()
        && let Ok(document) = store::read_json(&coverage)
        && let Some(values) = document.get("instruments").and_then(Value::as_array)
        && !values.is_empty()
    {
        return Ok(values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect());
    }
    Ok(INSTRUMENTS
        .iter()
        .map(|value| (*value).to_owned())
        .collect())
}

fn resolve_timeframe(
    value: Option<&str>,
    cell_dir: &Path,
    selection: Option<&str>,
) -> Result<Option<i64>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value == "auto" {
        let path = selection.map_or_else(
            || {
                cell_dir
                    .parent()
                    .and_then(Path::parent)
                    .unwrap_or(cell_dir)
                    .join("selection.json")
            },
            PathBuf::from,
        );
        let document = store::read_json(&path)?;
        let timeframe = document
            .get("timeframe_secs")
            .and_then(Value::as_i64)
            .filter(|value| *value > 0)
            .ok_or_else(|| format!("{} has no positive timeframe_secs", path.display()))?;
        return Ok(Some(timeframe));
    }
    value
        .parse::<i64>()
        .map(Some)
        .map_err(|_| format!("--timeframe must be seconds or auto, got {value}"))
}

fn cell_spec(cell_dir: &Path, instrument: &str, timeframe: Option<i64>) -> PathBuf {
    let name = timeframe.map_or_else(
        || format!("spec-{instrument}.json"),
        |timeframe| format!("spec-{instrument}-{timeframe}.json"),
    );
    cell_dir.join(name)
}

fn prepare_run_config(
    run_path: &Path,
    model_path: Option<&Path>,
    out_dir: &Path,
) -> Result<PathBuf, String> {
    let mut run = store::read_json(run_path)?;
    let base = run_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(out_dir)
        .map_err(|error| format!("cannot create {}: {error}", out_dir.display()))?;
    let data_path = absolute(Path::new(run["data"].as_str().unwrap_or("")), base);
    let mut data = store::read_json(&data_path)?;
    if let Some(source) = data.get_mut("source").and_then(Value::as_object_mut)
        && let Some(path) = source.get("path").and_then(Value::as_str)
    {
        let resolved = absolute(Path::new(path), data_path.parent().unwrap_or(base));
        source.insert("path".to_owned(), json!(resolved.display().to_string()));
    }
    store::write_json(&out_dir.join("data.json"), &data)?;
    for key in ["execution", "portfolio", "policy"] {
        let sibling = absolute(Path::new(run[key].as_str().unwrap_or("")), base);
        fs::copy(&sibling, out_dir.join(format!("{key}.json")))
            .map_err(|error| format!("cannot copy {}: {error}", sibling.display()))?;
    }
    let model_source = model_path.map_or_else(
        || absolute(Path::new(run["model"].as_str().unwrap_or("")), base),
        Path::to_path_buf,
    );
    fs::copy(&model_source, out_dir.join("model.json"))
        .map_err(|error| format!("cannot copy {}: {error}", model_source.display()))?;
    run["data"] = json!("data.json");
    run["model"] = json!("model.json");
    run["execution"] = json!("execution.json");
    run["portfolio"] = json!("portfolio.json");
    run["policy"] = json!("policy.json");
    run["output_root"] = json!(out_dir.parent().unwrap_or(out_dir).display().to_string());
    store::write_json(&out_dir.join("run.json"), &run)?;
    Ok(out_dir.join("run.json"))
}

fn count_bundle(
    bundle: &Path,
    timeframe: i64,
    gate: Option<&model_prediction::StateGate>,
    horizon: i64,
) -> Result<BTreeMap<String, usize>, String> {
    let (header, rows) = read_csv(&bundle.join("readings.csv"))?;
    let index = |name: &str| header.iter().position(|field| field == name);
    let (Some(tf), Some(bar)) = (index("timeframe_secs"), index("bar")) else {
        return Err("readings.csv has no timeframe/bar".to_owned());
    };
    let mut state_by_bar = BTreeMap::new();
    for row in rows {
        if row.get(tf).and_then(|value| value.parse::<i64>().ok()) != Some(timeframe) {
            continue;
        }
        let Some(bar) = row.get(bar).and_then(|value| value.parse::<i64>().ok()) else {
            continue;
        };
        let fields: BTreeMap<String, String> =
            header.iter().cloned().zip(row.iter().cloned()).collect();
        state_by_bar.insert(
            bar,
            gate.is_none_or(|gate| model_prediction::state_matches_csv(&fields, gate)),
        );
    }
    let (header, rows) = read_csv(&bundle.join("prediction.csv"))?;
    let index = |name: &str| header.iter().position(|field| field == name);
    let (Some(tf), Some(bar), Some(prediction)) =
        (index("timeframe_secs"), index("bar"), index("prediction"))
    else {
        return Err("prediction.csv has no timeframe/bar/prediction".to_owned());
    };
    let mut bars = Vec::new();
    for row in rows {
        if row.get(tf).and_then(|value| value.parse::<i64>().ok()) != Some(timeframe) {
            continue;
        }
        let (Some(bar), Some(prediction)) = (
            row.get(bar).and_then(|value| value.parse::<i64>().ok()),
            row.get(prediction)
                .and_then(|value| value.parse::<i64>().ok()),
        ) else {
            continue;
        };
        if prediction != 0 && state_by_bar.get(&bar).copied().unwrap_or(false) {
            bars.push(bar);
        }
    }
    bars.sort();
    let mut clusters = 0;
    let mut previous = None;
    for bar in &bars {
        if previous.is_none_or(|previous| *bar - previous >= horizon) {
            clusters += 1;
            previous = Some(*bar);
        }
    }
    let mut result = BTreeMap::new();
    result.insert("n".to_owned(), 0);
    result.insert("clusters".to_owned(), clusters);
    result.insert("n".to_owned(), bars.len());
    Ok(result)
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{} has no header", path.display()))?
        .split(',')
        .map(|value| value.trim().trim_matches('"').to_owned())
        .collect();
    Ok((
        header,
        lines
            .map(|line| {
                line.split(',')
                    .map(|value| value.trim().trim_matches('"').to_owned())
                    .collect()
            })
            .collect(),
    ))
}

fn file_digest(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
}

fn absolute(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn root(name: &str) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "qr-model-cell-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("tree");
        path
    }

    #[test]
    fn count_bundle_honours_nonzero_predictions_and_clusters() {
        let root = root("count");
        fs::write(root.join("readings.csv"), "timeframe_secs,bar,timestamp,close,atr,adx_regime\n300,0,1,100,1,1\n300,1,2,100,1,1\n300,2,3,100,1,1\n").expect("readings");
        fs::write(
            root.join("prediction.csv"),
            "timeframe_secs,bar,timestamp,prediction\n300,0,1,1\n300,1,2,0\n300,2,3,1\n",
        )
        .expect("prediction");
        let counts = count_bundle(&root, 300, None, 2).expect("count");
        assert_eq!(counts["n"], 2);
        assert_eq!(counts["clusters"], 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn requested_instruments_are_split() {
        assert_eq!(
            cell_instruments(Path::new("."), Some("XAU, BTC")).expect("instruments"),
            vec!["XAU", "BTC"]
        );
    }
}
