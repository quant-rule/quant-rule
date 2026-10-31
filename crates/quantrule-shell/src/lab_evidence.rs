//! The lab evidence shared by `lab summary` and `lab diagnose`.
//!
//! Both commands report whether a lab has out-of-sample evidence. The
//! comparison lives here so the two renderers cannot disagree about what
//! counts as out of sample: a different `data_version`, not merely an `oos/`
//! folder.

use std::fs;
use std::path::Path;

use quantrule_analytics::diagnosis::types::OutOfSampleStatus;
use serde_json::Value;

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

fn read_json(path: &Path) -> Option<Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}
