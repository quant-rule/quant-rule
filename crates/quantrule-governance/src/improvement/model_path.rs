//! Descriptive path and expectancy evaluation for a model cell.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::store;

const SCHEMA: &str = "quantrule.model-path/v1";
const DEFAULT_MENU: [(f64, f64, i64); 3] = [(2.0, 1.0, 20), (1.0, 1.0, 20), (3.0, 1.0, 40)];

#[derive(Clone, PartialEq)]
struct Menu {
    target_atr: f64,
    stop_atr: f64,
    horizon: i64,
}

struct PathRow {
    menu: Menu,
    mfe_atr: f64,
    mae_atr: f64,
    exit_r: f64,
    time_to_target: Option<i64>,
    time_to_stop: Option<i64>,
}

struct Bundle {
    name: String,
    readings_header: Vec<String>,
    readings: Vec<Vec<String>>,
    predictions_header: Vec<String>,
    predictions: Vec<Vec<String>>,
}

/// Run the model-path evaluation.
///
/// # Errors
///
/// Returns a message for a missing bundle, an invalid menu, or a write failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let mut bundles = Vec::new();
    let mut spec_id = None;
    let mut family = None;
    let mut partition = None;
    let mut condition_column = None;
    let mut menu_path = None;
    let mut cost_bps = 6.0;
    let mut out = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bundle" => {
                let value = value(args, &mut index, "--bundle")?;
                let Some((name, path)) = value.split_once('=') else {
                    return Err(format!("--bundle must be name=path, got {value:?}"));
                };
                bundles.push((name.to_owned(), PathBuf::from(path)));
            }
            "--spec-id" => spec_id = Some(value(args, &mut index, "--spec-id")?),
            "--family" => family = Some(value(args, &mut index, "--family")?),
            "--partition" => partition = Some(value(args, &mut index, "--partition")?),
            "--condition-column" => {
                condition_column = Some(value(args, &mut index, "--condition-column")?)
            }
            "--menu" => menu_path = Some(value(args, &mut index, "--menu")?),
            "--cost-bps" => {
                cost_bps = value(args, &mut index, "--cost-bps")?
                    .parse()
                    .map_err(|_| "--cost-bps must be a number".to_owned())?;
            }
            "--out" => out = Some(value(args, &mut index, "--out")?),
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let out = PathBuf::from(out.ok_or_else(|| "--out is required".to_owned())?);
    if bundles.is_empty() {
        return Err("--bundle is required".to_owned());
    }
    let menu = parse_menu(menu_path.as_deref())?;
    let mut loaded = Vec::new();
    for (name, path) in bundles {
        let (readings_header, readings) = read_csv(&path.join("readings.csv"))?;
        let (predictions_header, predictions) = read_csv(&path.join("prediction.csv"))?;
        loaded.push(Bundle {
            name,
            readings_header,
            readings,
            predictions_header,
            predictions,
        });
    }
    let mut instruments = Map::new();
    let mut all_rows: Vec<PathRow> = Vec::new();
    for bundle in &loaded {
        let by_bar = readings_by_bar(&bundle.readings_header, &bundle.readings);
        let series = series(&bundle.readings_header, &bundle.readings);
        let mut bars = Vec::new();
        for prediction in &bundle.predictions {
            let bar = field_int(&bundle.predictions_header, prediction, "bar");
            let direction = field_int(&bundle.predictions_header, prediction, "prediction");
            let (Some(bar), Some(direction)) = (bar, direction) else {
                continue;
            };
            if direction != 1 && direction != -1 {
                continue;
            }
            let Some(reading) = by_bar.get(&bar) else {
                continue;
            };
            if let Some(column) = &condition_column
                && field_int(&bundle.readings_header, reading, column) != Some(1)
            {
                continue;
            }
            let Some(close) = field_float(&bundle.readings_header, reading, "close") else {
                continue;
            };
            let Some(atr) = field_float(&bundle.readings_header, reading, "atr") else {
                continue;
            };
            if !close.is_finite() || !atr.is_finite() || atr <= 0.0 {
                continue;
            }
            let mut paths = Vec::new();
            for item in &menu {
                if let Some(path) = path_for(&series, bar, direction, item, cost_bps) {
                    paths.push(path);
                }
            }
            if !paths.is_empty() {
                bars.push(bar);
                all_rows.extend(paths);
            }
        }
        instruments.insert(bundle.name.clone(), json!({"n": bars.len(), "bars": bars}));
    }
    let mut summaries = Vec::new();
    for item in &menu {
        let selected: Vec<&PathRow> = all_rows.iter().filter(|row| row.menu == *item).collect();
        let summary = summarize(&selected);
        summaries.push(json!({
            "menu": {"target_atr": item.target_atr, "stop_atr": item.stop_atr, "horizon": item.horizon},
            "key": format!("{}/{}x{}", format_float(item.target_atr), format_float(item.stop_atr), item.horizon),
            "summary": summary,
        }));
    }
    let document = json!({
        "schema": SCHEMA,
        "spec_id": spec_id,
        "family": family,
        "partition": partition,
        "cost_bps": cost_bps,
        "condition_column": condition_column,
        "instruments": instruments,
        "menu": menu.iter().map(|item| json!({"target_atr": item.target_atr, "stop_atr": item.stop_atr, "horizon": item.horizon})).collect::<Vec<_>>(),
        "summaries": summaries,
    });
    fs::create_dir_all(&out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let text = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("cannot encode path.json: {error}"))?;
    fs::write(out.join("path.json"), format!("{text}\n"))
        .map_err(|error| format!("cannot write path.json: {error}"))?;
    let mut lines = vec![
        "# Model Path and Expectancy".to_owned(),
        String::new(),
        format!("- Spec: `{}`", spec_id.as_deref().unwrap_or("None")),
        format!("- Partition: `{}`", partition.as_deref().unwrap_or("None")),
        format!(
            "- Condition column: `{}`",
            condition_column.as_deref().unwrap_or("None")
        ),
        format!("- Cost: {} bps", format_float(cost_bps)),
        String::new(),
        "| Exit | n | Mean net R | Median net R | Win rate | Mean MFE | Mean MAE | Targets | Stops | Timeouts |".to_owned(),
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|".to_owned(),
    ];
    for summary in &summaries {
        let key = summary["key"].as_str().unwrap_or("");
        let values = &summary["summary"];
        lines.push(format!(
            "| {key} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            values
                .get("n")
                .map_or_else(|| "0".to_owned(), display_value),
            values
                .get("mean_net_r")
                .map_or_else(|| "—".to_owned(), display_value),
            values
                .get("median_net_r")
                .map_or_else(|| "—".to_owned(), display_value),
            values
                .get("win_rate")
                .map_or_else(|| "—".to_owned(), display_value),
            values
                .get("mean_mfe_atr")
                .map_or_else(|| "—".to_owned(), display_value),
            values
                .get("mean_mae_atr")
                .map_or_else(|| "—".to_owned(), display_value),
            values
                .get("target_count")
                .map_or_else(|| "—".to_owned(), display_value),
            values
                .get("stop_count")
                .map_or_else(|| "—".to_owned(), display_value),
            values
                .get("timeout_count")
                .map_or_else(|| "—".to_owned(), display_value),
        ));
    }
    lines.push(String::new());
    fs::write(out.join("path.md"), lines.join("\n"))
        .map_err(|error| format!("cannot write path.md: {error}"))?;
    println!(
        "ok path spec={} menu={} -> {}",
        spec_id.as_deref().unwrap_or(""),
        menu.len(),
        out.display()
    );
    Ok(())
}

fn path_for(
    rows: &[(f64, f64, f64, f64)],
    index: i64,
    direction: i64,
    menu: &Menu,
    cost_bps: f64,
) -> Option<PathRow> {
    let index = usize::try_from(index).ok()?;
    if index + menu.horizon as usize >= rows.len() {
        return None;
    }
    let (_, _, entry, atr) = rows[index];
    if atr <= 0.0 {
        return None;
    }
    let target = entry + direction as f64 * menu.target_atr * atr;
    let stop = entry - direction as f64 * menu.stop_atr * atr;
    let mut mfe = 0.0;
    let mut mae = 0.0;
    let mut target_time = None;
    let mut stop_time = None;
    let mut exit_r = None;
    for offset in 1..=menu.horizon as usize {
        let (high, low, _, _) = rows[index + offset];
        let (favorable, adverse) = if direction > 0 {
            ((high - entry) / atr, (low - entry) / atr)
        } else {
            ((entry - low) / atr, (entry - high) / atr)
        };
        mfe = f64::max(mfe, favorable);
        mae = f64::min(mae, adverse);
        let (hit_stop, hit_target) = if direction > 0 {
            (low <= stop, high >= target)
        } else {
            (high >= stop, low <= target)
        };
        if hit_stop {
            stop_time = Some(offset as i64);
            exit_r = Some(-menu.stop_atr);
            break;
        }
        if hit_target {
            target_time = Some(offset as i64);
            exit_r = Some(menu.target_atr);
            break;
        }
    }
    let cost = cost_bps / 10_000.0 * entry / atr;
    let exit_r = exit_r.unwrap_or_else(|| {
        direction as f64 * (rows[index + menu.horizon as usize].2 - entry) / atr
    });
    Some(PathRow {
        menu: menu.clone(),
        mfe_atr: mfe,
        mae_atr: mae,
        exit_r: exit_r - cost,
        time_to_target: target_time,
        time_to_stop: stop_time,
    })
}

fn summarize(rows: &[&PathRow]) -> Value {
    if rows.is_empty() {
        return json!({"n": 0});
    }
    let net: Vec<f64> = rows.iter().map(|row| row.exit_r).collect();
    let mfe: Vec<f64> = rows.iter().map(|row| row.mfe_atr).collect();
    let mae: Vec<f64> = rows.iter().map(|row| row.mae_atr).collect();
    let targets: Vec<f64> = rows
        .iter()
        .filter_map(|row| row.time_to_target.map(|value| value as f64))
        .collect();
    let stops: Vec<f64> = rows
        .iter()
        .filter_map(|row| row.time_to_stop.map(|value| value as f64))
        .collect();
    json!({
        "n": rows.len(),
        "mean_net_r": mean(&net),
        "median_net_r": median(&net),
        "win_rate": net.iter().filter(|value| **value > 0.0).count() as f64 / net.len() as f64,
        "mean_mfe_atr": mean(&mfe),
        "median_mfe_atr": median(&mfe),
        "p10_mfe_atr": quantile(&mfe, 0.10),
        "p90_mfe_atr": quantile(&mfe, 0.90),
        "mean_mae_atr": mean(&mae),
        "median_mae_atr": median(&mae),
        "p10_mae_atr": quantile(&mae, 0.10),
        "p90_mae_atr": quantile(&mae, 0.90),
        "target_count": targets.len(),
        "stop_count": stops.len(),
        "timeout_count": rows.len() - targets.len() - stops.len(),
        "mean_time_to_target": mean(&targets),
        "mean_time_to_stop": mean(&stops),
    })
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| left.total_cmp(right));
    let middle = ordered.len() / 2;
    if ordered.len() % 2 == 1 {
        Some(ordered[middle])
    } else {
        Some((ordered[middle - 1] + ordered[middle]) / 2.0)
    }
}

fn quantile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| left.total_cmp(right));
    let index = ((fraction * (ordered.len() - 1) as f64) as usize).min(ordered.len() - 1);
    Some(ordered[index])
}

fn parse_menu(path: Option<&str>) -> Result<Vec<Menu>, String> {
    let values = if let Some(path) = path {
        let document = store::read_json(Path::new(path))?;
        document
            .as_array()
            .filter(|items| !items.is_empty())
            .cloned()
            .ok_or_else(|| "menu must be a non-empty JSON list".to_owned())?
    } else {
        DEFAULT_MENU
            .iter()
            .map(|(target, stop, horizon)| {
                json!({"target_atr": target, "stop_atr": stop, "horizon": horizon})
            })
            .collect()
    };
    let mut menu = Vec::new();
    for item in values {
        let Some(item) = item.as_object() else {
            return Err("menu item must be an object".to_owned());
        };
        let target = item.get("target_atr").and_then(Value::as_f64);
        let stop = item.get("stop_atr").and_then(Value::as_f64);
        let horizon = item.get("horizon").and_then(Value::as_i64);
        let (Some(target), Some(stop), Some(horizon)) = (target, stop, horizon) else {
            return Err("menu item needs target_atr, stop_atr, horizon".to_owned());
        };
        if target <= 0.0 || stop <= 0.0 || horizon <= 0 {
            return Err("menu values must be positive".to_owned());
        }
        menu.push(Menu {
            target_atr: target,
            stop_atr: stop,
            horizon,
        });
    }
    Ok(menu)
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .unwrap_or("")
        .split(',')
        .map(|value| value.trim().trim_matches('"').to_owned())
        .collect();
    let rows = lines
        .map(|line| {
            line.split(',')
                .map(|value| value.trim().trim_matches('"').to_owned())
                .collect()
        })
        .collect();
    Ok((header, rows))
}

fn readings_by_bar(header: &[String], rows: &[Vec<String>]) -> BTreeMap<i64, Vec<String>> {
    let Some(index) = header.iter().position(|field| field == "bar") else {
        return BTreeMap::new();
    };
    rows.iter()
        .filter_map(|row| Some((row.get(index)?.parse().ok()?, row.clone())))
        .collect()
}

fn series(header: &[String], rows: &[Vec<String>]) -> Vec<(f64, f64, f64, f64)> {
    let close = header.iter().position(|field| field == "close");
    let high = header.iter().position(|field| field == "high");
    let low = header.iter().position(|field| field == "low");
    let atr = header.iter().position(|field| field == "atr");
    rows.iter()
        .map(|row| {
            (
                high.and_then(|index| field(row, index)).unwrap_or(f64::NAN),
                low.and_then(|index| field(row, index)).unwrap_or(f64::NAN),
                close
                    .and_then(|index| field(row, index))
                    .unwrap_or(f64::NAN),
                atr.and_then(|index| field(row, index)).unwrap_or(f64::NAN),
            )
        })
        .collect()
}

fn field(values: &[String], index: usize) -> Option<f64> {
    values.get(index).and_then(|value| value.parse().ok())
}

fn field_float(header: &[String], row: &[String], name: &str) -> Option<f64> {
    header
        .iter()
        .position(|field| field == name)
        .and_then(|index| field(row, index))
}

fn field_int(header: &[String], row: &[String], name: &str) -> Option<i64> {
    let value = field_float(header, row, name)?;
    value.is_finite().then_some(value as i64)
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => value.to_string(),
    }
}

fn format_float(value: f64) -> String {
    serde_json::Number::from_f64(value)
        .map_or_else(|| value.to_string(), |number| number.to_string())
}

fn value(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} needs a value"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn root(name: &str) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "qr-model-path-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("tree");
        path
    }

    fn bundle(root: &Path) -> PathBuf {
        let bundle = root.join("bundle");
        fs::create_dir_all(&bundle).expect("bundle");
        let mut readings =
            String::from("timeframe_secs,bar,timestamp,open,high,low,close,volume,atr\n");
        for bar in 0..30 {
            let close = 100.0 + bar as f64 * 0.1;
            readings.push_str(&format!(
                "900,{bar},{},{close},{},{},{close},1.0,1.0\n",
                1_700_000_000 + bar * 900,
                close + 0.5,
                close - 0.5
            ));
        }
        fs::write(bundle.join("readings.csv"), readings).expect("readings");
        fs::write(
            bundle.join("prediction.csv"),
            "timeframe_secs,bar,timestamp,prediction,long_votes,short_votes\n900,0,1700000000,1,1,0\n",
        )
        .expect("prediction");
        bundle
    }

    fn menu(root: &Path) -> PathBuf {
        let path = root.join("menu.json");
        fs::write(&path, r#"[{"target_atr":2.0,"stop_atr":1.0,"horizon":25}]"#).expect("menu");
        path
    }

    #[test]
    fn a_reached_target_is_counted() {
        let root = root("target");
        let bundle = bundle(&root);
        let menu = menu(&root);
        let out = root.join("out");
        run(&[
            "--bundle".to_owned(),
            format!("XAU={}", bundle.display()),
            "--spec-id".to_owned(),
            "MR5.test".to_owned(),
            "--partition".to_owned(),
            "model_dev".to_owned(),
            "--menu".to_owned(),
            menu.display().to_string(),
            "--out".to_owned(),
            out.display().to_string(),
        ])
        .expect("run");
        let document: Value =
            serde_json::from_str(&fs::read_to_string(out.join("path.json")).expect("json"))
                .expect("parse");
        let summary = &document["summaries"][0]["summary"];
        assert_eq!(summary["n"], 1);
        assert_eq!(summary["target_count"], 1);
        assert_eq!(summary["stop_count"], 0);
        assert!(summary["mean_net_r"].as_f64().expect("mean") > 0.0);
        assert!(summary["mean_mfe_atr"].as_f64().expect("mfe") >= 2.0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn the_output_is_deterministic() {
        let root = root("deterministic");
        let bundle = bundle(&root);
        let menu = menu(&root);
        let first = root.join("first");
        let second = root.join("second");
        for out in [&first, &second] {
            run(&[
                "--bundle".to_owned(),
                format!("XAU={}", bundle.display()),
                "--spec-id".to_owned(),
                "MR5.test".to_owned(),
                "--menu".to_owned(),
                menu.display().to_string(),
                "--out".to_owned(),
                out.display().to_string(),
            ])
            .expect("run");
        }
        assert_eq!(
            fs::read(first.join("path.json")).expect("first"),
            fs::read(second.join("path.json")).expect("second")
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
