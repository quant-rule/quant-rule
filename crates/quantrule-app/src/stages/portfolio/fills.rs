//! Read an execution fills bundle into the portfolio vocabulary.
//!
//! The reader consumes `fills.csv` and `provenance.json`. It does not read an
//! order, a strategy decision, a model bundle, cash, equity, or P&L. The fills
//! artifact is the handoff between execution and portfolio.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::artifact::provenance::{ProvenanceError, StageProvenance};
use quantrule_backtest::execution::{Fill, FillReason, OrderSide};

/// The execution fills bundle could not be read.
#[derive(Debug)]
pub enum FillsError {
    /// A required artifact could not be read.
    Read {
        /// The artifact path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A required artifact is not shaped like a fills bundle.
    Invalid {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong, with the fill's own field names.
        message: String,
    },
}

impl fmt::Display for FillsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "invalid fills bundle {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for FillsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

/// A complete fills bundle.
#[derive(Debug, Clone)]
pub struct FillsBundle {
    /// The execution output's data version.
    pub data_version: String,
    /// The execution configuration identity.
    pub config_id: String,
    /// The requested period start, as written.
    pub period_from: Option<String>,
    /// The requested period end, as written.
    pub period_to: Option<String>,
    /// The fills in fill order.
    pub fills: Vec<Fill>,
}

impl FillsBundle {
    /// Loads a fills bundle from its directory.
    ///
    /// # Errors
    ///
    /// Returns [`FillsError`] when a required artifact is missing, unreadable,
    /// or not shaped like a fills bundle.
    pub fn load(path: &Path) -> Result<Self, FillsError> {
        let provenance = parse_provenance(&path.join("provenance.json"))?;
        let fills = parse_fills(&path.join("fills.csv"))?;
        Ok(Self {
            data_version: provenance.data_version,
            config_id: provenance.config_id,
            period_from: provenance.period_from,
            period_to: provenance.period_to,
            fills,
        })
    }
}

fn parse_provenance(path: &Path) -> Result<StageProvenance, FillsError> {
    crate::artifact::provenance::load_stage(path, "execution", "fills").map_err(fills_error)
}

fn fills_error(error: ProvenanceError) -> FillsError {
    match error {
        ProvenanceError::Read { path, source } => FillsError::Read { path, source },
        ProvenanceError::Invalid { path, message } => FillsError::Invalid { path, message },
    }
}

fn parse_fills(path: &Path) -> Result<Vec<Fill>, FillsError> {
    let text = read_text(path)?;
    let mut lines = text.lines();
    let header = lines.next().ok_or_else(|| FillsError::Invalid {
        path: path.to_path_buf(),
        message: "the file is empty".to_owned(),
    })?;
    let header: Vec<&str> = header.split(',').collect();
    let column = |name: &str| -> Result<usize, FillsError> {
        header
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| FillsError::Invalid {
                path: path.to_path_buf(),
                message: format!("the required column '{name}' is missing"),
            })
    };
    let fill_id = column("fill_id")?;
    let side = column("side")?;
    let quantity = column("quantity")?;
    let reference_price = column("reference_price")?;
    let price = column("price")?;
    let slippage = column("slippage")?;
    let fee = column("fee")?;
    let bar = column("bar")?;
    let time = column("time")?;
    let reason = column("reason")?;

    let mut fills = Vec::new();
    for (number, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != header.len() {
            return Err(FillsError::Invalid {
                path: path.to_path_buf(),
                message: format!(
                    "row {} has {} fields; the header has {}",
                    number + 2,
                    fields.len(),
                    header.len()
                ),
            });
        }
        let expected_id = number as u64;
        let id = parse_u64(fields[fill_id], path, number, "fill_id")?;
        if id != expected_id {
            return Err(FillsError::Invalid {
                path: path.to_path_buf(),
                message: format!("fill {number} has id {id}; expected {expected_id}"),
            });
        }
        let fill = Fill::new(
            parse_side(fields[side], path, number)?,
            parse_f64(fields[quantity], path, number, "quantity")?,
            parse_f64(fields[reference_price], path, number, "reference_price")?,
            parse_f64(fields[fee], path, number, "fee")?,
            parse_f64(fields[slippage], path, number, "slippage")?,
            parse_usize(fields[bar], path, number, "bar")?,
            parse_i64(fields[time], path, number, "time")?,
            parse_reason(fields[reason], path, number)?,
        )
        .map_err(|error| FillsError::Invalid {
            path: path.to_path_buf(),
            message: format!("fill {number} is invalid: {error}"),
        })?;
        let stored_price = parse_f64(fields[price], path, number, "price")?;
        if (fill.price - stored_price).abs() > 1e-9 {
            return Err(FillsError::Invalid {
                path: path.to_path_buf(),
                message: format!(
                    "fill {number} stores price {stored_price}, but reference plus slippage is {}",
                    fill.price
                ),
            });
        }
        fills.push(fill);
    }
    Ok(fills)
}

fn read_text(path: &Path) -> Result<String, FillsError> {
    fs::read_to_string(path).map_err(|source| FillsError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_side(field: &str, path: &Path, number: usize) -> Result<OrderSide, FillsError> {
    match field {
        "buy" => Ok(OrderSide::Buy),
        "sell" => Ok(OrderSide::Sell),
        other => Err(FillsError::Invalid {
            path: path.to_path_buf(),
            message: format!("fill {number} has unknown side '{other}'"),
        }),
    }
}

fn parse_reason(field: &str, path: &Path, number: usize) -> Result<FillReason, FillsError> {
    match field {
        "entry" => Ok(FillReason::Entry),
        "exit" => Ok(FillReason::Exit),
        "stop_loss" => Ok(FillReason::StopLoss),
        "take_profit" => Ok(FillReason::TakeProfit),
        other => Err(FillsError::Invalid {
            path: path.to_path_buf(),
            message: format!("fill {number} has unknown reason '{other}'"),
        }),
    }
}

fn parse_f64(field: &str, path: &Path, number: usize, name: &str) -> Result<f64, FillsError> {
    field.parse::<f64>().map_err(|_| FillsError::Invalid {
        path: path.to_path_buf(),
        message: format!("fill {number} has invalid {name} {field:?}"),
    })
}

fn parse_u64(field: &str, path: &Path, number: usize, name: &str) -> Result<u64, FillsError> {
    field.parse::<u64>().map_err(|_| FillsError::Invalid {
        path: path.to_path_buf(),
        message: format!("fill {number} has invalid {name} {field:?}"),
    })
}

fn parse_usize(field: &str, path: &Path, number: usize, name: &str) -> Result<usize, FillsError> {
    field.parse::<usize>().map_err(|_| FillsError::Invalid {
        path: path.to_path_buf(),
        message: format!("fill {number} has invalid {name} {field:?}"),
    })
}

fn parse_i64(field: &str, path: &Path, number: usize, name: &str) -> Result<i64, FillsError> {
    field.parse::<i64>().map_err(|_| FillsError::Invalid {
        path: path.to_path_buf(),
        message: format!("fill {number} has invalid {name} {field:?}"),
    })
}

#[cfg(test)]
mod tests {
    use super::FillsBundle;

    #[test]
    fn a_missing_bundle_is_reported() {
        let error = FillsBundle::load(std::path::Path::new("target/no-such-fills"))
            .expect_err("the bundle does not exist");
        assert!(error.to_string().contains("cannot read"));
    }
}
