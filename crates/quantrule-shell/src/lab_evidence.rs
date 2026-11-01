//! The lab evidence shared by `lab summary` and `lab diagnose`.
//!
//! Both commands report whether a lab has out-of-sample and independent-rerun
//! evidence. The comparisons live here so the two renderers cannot disagree
//! about what counts: an `oos/` report needs a different `data_version`, and a
//! `rerun/` report needs the main lab's `config_hash` plus the five reproduced
//! summary fields.

use std::fs;
use std::path::Path;

use quantrule_analytics::diagnosis::types::OutOfSampleStatus;
use serde_json::Value;

/// The status of a lab's independent-rerun evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RerunStatus {
    /// No `rerun/portfolio/report.json` exists.
    Absent,
    /// A rerun report or provenance is missing, malformed, or lacks a field.
    Unreadable,
    /// The rerun exists but its config hash or reproduced fields differ.
    Differs,
    /// The rerun has the main config hash and reproduces every summary field.
    Matches,
}

impl RerunStatus {
    /// Whether the rerun is independent matching evidence.
    pub(crate) const fn is_match(self) -> bool {
        matches!(self, Self::Matches)
    }
}

/// Reads the out-of-sample evidence a lab folder records.
///
/// An `oos/` folder counts only when its portfolio report is readable and its
/// `data_version` differs from the main lab's. An unreadable or version-less
/// report is [`OutOfSampleStatus::Unreadable`], not absent.
pub(crate) fn out_of_sample(lab_dir: &Path) -> OutOfSampleStatus {
    let oos = lab_dir.join("oos").join("portfolio").join("report.json");
    if !oos.is_file() {
        return OutOfSampleStatus::Absent;
    }
    let main = lab_dir.join("portfolio").join("report.json");
    let (Some(oos), Some(main)) = (read_json(&oos), read_json(&main)) else {
        return OutOfSampleStatus::Unreadable;
    };
    let (Some(oos_version), Some(main_version)) = (
        oos.get("data_version").and_then(Value::as_str),
        main.get("data_version").and_then(Value::as_str),
    ) else {
        return OutOfSampleStatus::Unreadable;
    };
    if oos_version == main_version {
        OutOfSampleStatus::SameDataVersion
    } else {
        OutOfSampleStatus::DistinctDataVersion
    }
}

/// Reads the independent-rerun evidence a lab folder records.
///
/// A rerun counts only when both portfolio reports and both provenance files
/// are readable, the config hashes match, and the five summary fields match.
pub(crate) fn rerun(lab_dir: &Path) -> RerunStatus {
    let rerun_path = lab_dir.join("rerun").join("portfolio").join("report.json");
    if !rerun_path.is_file() {
        return RerunStatus::Absent;
    }
    let (Some(rerun), Some(main)) = (
        read_json(&rerun_path),
        read_json(&lab_dir.join("portfolio").join("report.json")),
    ) else {
        return RerunStatus::Unreadable;
    };
    let (Some(rerun_hash), Some(main_hash)) = (
        read_config_hash(
            &lab_dir
                .join("rerun")
                .join("portfolio")
                .join("provenance.json"),
        ),
        read_config_hash(&lab_dir.join("portfolio").join("provenance.json")),
    ) else {
        return RerunStatus::Unreadable;
    };
    if rerun_hash != main_hash {
        return RerunStatus::Differs;
    }
    for field in [
        "final_equity",
        "net_pnl",
        "trades",
        "max_drawdown",
        "halted",
    ] {
        let (Some(left), Some(right)) = (
            rerun.pointer(&format!("/summary/{field}")),
            main.pointer(&format!("/summary/{field}")),
        ) else {
            return RerunStatus::Unreadable;
        };
        if left != right {
            return RerunStatus::Differs;
        }
    }
    RerunStatus::Matches
}

fn read_config_hash(path: &Path) -> Option<String> {
    read_json(path)?
        .get("config_hash")?
        .as_str()
        .map(str::to_owned)
}

fn read_json(path: &Path) -> Option<Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::{Value, json};

    use quantrule_analytics::diagnosis::types::OutOfSampleStatus;

    use super::{RerunStatus, out_of_sample, rerun};

    fn lab(name: &str) -> PathBuf {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-artifacts/lab-evidence")
            .join(name);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the lab directory is created");
        path
    }

    fn write(path: &Path, value: &Value) {
        fs::create_dir_all(path.parent().expect("the path has a parent"))
            .expect("the parent directory is created");
        fs::write(
            path,
            serde_json::to_vec_pretty(value).expect("the value is serializable"),
        )
        .expect("the value is written");
    }

    fn report(equity: f64) -> Value {
        json!({
            "data_version": "test:v1",
            "summary": {
                "final_equity": equity,
                "net_pnl": 1.0,
                "trades": 1,
                "max_drawdown": 0.1,
                "halted": false
            }
        })
    }

    fn provenance(hash: &str) -> Value {
        json!({"config_hash": hash})
    }

    fn main_lab(lab: &Path) -> PathBuf {
        let portfolio = lab.join("portfolio");
        write(&portfolio.join("report.json"), &report(100.0));
        write(
            &portfolio.join("provenance.json"),
            &provenance("aaaaaaaaaaaaaaaa"),
        );
        portfolio
    }

    #[test]
    fn out_of_sample_is_absent_without_an_oos_report() {
        let lab = lab("oos-absent");
        main_lab(&lab);
        assert_eq!(out_of_sample(&lab), OutOfSampleStatus::Absent);
    }

    #[test]
    fn out_of_sample_is_unreadable_when_either_report_is_malformed() {
        let lab = lab("oos-unreadable");
        main_lab(&lab);
        let oos = lab.join("oos").join("portfolio");
        fs::create_dir_all(&oos).expect("the oos directory is created");
        fs::write(oos.join("report.json"), b"not json").expect("the malformed report is written");
        assert_eq!(out_of_sample(&lab), OutOfSampleStatus::Unreadable);
    }

    #[test]
    fn out_of_sample_compares_the_data_versions() {
        let lab = lab("oos-versions");
        main_lab(&lab);
        let oos = lab.join("oos").join("portfolio");
        write(&oos.join("report.json"), &report(100.0));
        assert_eq!(out_of_sample(&lab), OutOfSampleStatus::SameDataVersion);

        let mut distinct = report(100.0);
        distinct["data_version"] = json!("test:v2");
        write(&oos.join("report.json"), &distinct);
        assert_eq!(out_of_sample(&lab), OutOfSampleStatus::DistinctDataVersion);
    }

    #[test]
    fn rerun_is_unreadable_when_its_report_is_malformed() {
        let lab = lab("rerun-malformed");
        main_lab(&lab);
        let rerun_dir = lab.join("rerun").join("portfolio");
        fs::create_dir_all(&rerun_dir).expect("the rerun directory is created");
        fs::write(rerun_dir.join("report.json"), b"not json")
            .expect("the malformed report is written");
        write(
            &rerun_dir.join("provenance.json"),
            &provenance("aaaaaaaaaaaaaaaa"),
        );
        assert_eq!(rerun(&lab), RerunStatus::Unreadable);
    }

    #[test]
    fn absent_when_there_is_no_rerun_report() {
        let lab = lab("absent");
        main_lab(&lab);
        assert_eq!(rerun(&lab), RerunStatus::Absent);
    }

    #[test]
    fn unreadable_when_the_rerun_provenance_is_missing() {
        let lab = lab("unreadable");
        main_lab(&lab);
        let rerun_dir = lab.join("rerun").join("portfolio");
        write(&rerun_dir.join("report.json"), &report(100.0));
        assert_eq!(rerun(&lab), RerunStatus::Unreadable);
    }

    #[test]
    fn differs_when_the_config_hashes_differ() {
        let lab = lab("different-config");
        main_lab(&lab);
        let rerun_dir = lab.join("rerun").join("portfolio");
        write(&rerun_dir.join("report.json"), &report(100.0));
        write(
            &rerun_dir.join("provenance.json"),
            &provenance("bbbbbbbbbbbbbbbb"),
        );
        assert_eq!(rerun(&lab), RerunStatus::Differs);
    }

    #[test]
    fn differs_when_a_summary_field_differs() {
        let lab = lab("different-result");
        main_lab(&lab);
        let rerun_dir = lab.join("rerun").join("portfolio");
        write(&rerun_dir.join("report.json"), &report(99.0));
        write(
            &rerun_dir.join("provenance.json"),
            &provenance("aaaaaaaaaaaaaaaa"),
        );
        assert_eq!(rerun(&lab), RerunStatus::Differs);
    }

    #[test]
    fn matches_when_the_config_and_fields_match() {
        let lab = lab("matching");
        main_lab(&lab);
        let rerun_dir = lab.join("rerun").join("portfolio");
        write(&rerun_dir.join("report.json"), &report(100.0));
        write(
            &rerun_dir.join("provenance.json"),
            &provenance("aaaaaaaaaaaaaaaa"),
        );
        assert_eq!(rerun(&lab), RerunStatus::Matches);
        assert!(rerun(&lab).is_match());
    }
}
