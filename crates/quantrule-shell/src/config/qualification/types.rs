//! The validated vocabulary of `qualification.json`.
//!
//! The types carry no parser behavior and no evidence value. The parser in
//! `parse.rs` builds them; the qualification stage reads them.

/// The qualification criteria schema.
pub(crate) const SCHEMA: &str = "quantrule.qualification/v1";

/// A fixed data partition declared by the improvement workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Partition {
    /// Model development, 2018-01-01..2020-12-31.
    ModelDev,
    /// Model validation, 2021-01-01..2022-12-31.
    ModelValidation,
    /// Strategy development, 2018-01-01..2022-12-31.
    StrategyDev,
    /// One-shot strategy validation, 2024-01-01..2024-12-31.
    StrategyValidation,
    /// Portfolio development, 2024-01-01..2024-12-31.
    PortfolioDev,
    /// Final portfolio holdout, 2025-01-01..2025-12-31.
    PortfolioHoldout,
}

impl Partition {
    /// Every declared partition, in workflow order.
    pub(crate) const ALL: [Self; 6] = [
        Self::ModelDev,
        Self::ModelValidation,
        Self::StrategyDev,
        Self::StrategyValidation,
        Self::PortfolioDev,
        Self::PortfolioHoldout,
    ];

    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::ModelDev => "model_dev",
            Self::ModelValidation => "model_validation",
            Self::StrategyDev => "strategy_dev",
            Self::StrategyValidation => "strategy_validation",
            Self::PortfolioDev => "portfolio_dev",
            Self::PortfolioHoldout => "portfolio_holdout",
        }
    }

    /// Parses a stable wire code.
    #[must_use]
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|partition| partition.code() == code)
    }

    /// The inclusive UTC window, as `YYYY-MM-DD` bounds.
    #[must_use]
    pub(crate) const fn window(self) -> (&'static str, &'static str) {
        match self {
            Self::ModelDev => ("2018-01-01", "2020-12-31"),
            Self::ModelValidation => ("2021-01-01", "2022-12-31"),
            Self::StrategyDev => ("2018-01-01", "2022-12-31"),
            Self::StrategyValidation => ("2024-01-01", "2024-12-31"),
            Self::PortfolioDev => ("2024-01-01", "2024-12-31"),
            Self::PortfolioHoldout => ("2025-01-01", "2025-12-31"),
        }
    }
}

/// The declared cost model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CostModel {
    /// Execution fees, slippage, commission, and funding are carried by the
    /// execution and portfolio artifacts.
    ExecutionAndPortfolio,
}

impl CostModel {
    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::ExecutionAndPortfolio => "execution-and-portfolio",
        }
    }

    /// Parses a stable wire code.
    #[must_use]
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "execution-and-portfolio" => Some(Self::ExecutionAndPortfolio),
            _ => None,
        }
    }
}

/// A declared multiple-testing correction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Correction {
    /// Holm's step-down correction.
    Holm,
    /// Bonferroni's single-step correction.
    Bonferroni,
    /// No correction.
    None,
}

impl Correction {
    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Holm => "holm",
            Self::Bonferroni => "bonferroni",
            Self::None => "none",
        }
    }

    /// Parses a stable wire code.
    #[must_use]
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "holm" => Some(Self::Holm),
            "bonferroni" => Some(Self::Bonferroni),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// The pre-registered multiple-testing declaration.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MultipleTesting {
    /// The number of primary cells.
    pub(crate) primary_cells: u64,
    /// The number of trials, at least the primary-cell count.
    pub(crate) trials: u64,
    /// The declared correction.
    pub(crate) correction: Correction,
    /// The family-wise alpha, in `(0, 1)`.
    pub(crate) family_alpha: f64,
}

impl MultipleTesting {
    /// The strictest adjusted alpha for the declared correction.
    ///
    /// The v1 result artifacts publish no p-value, so the engine records the
    /// first-step threshold rather than adjusting a value it does not have. For
    /// Holm and Bonferroni that threshold is `family_alpha / primary_cells`; for
    /// `none` it is the family alpha itself. A later artifact that publishes a
    /// p-value can compare it against this threshold without changing the
    /// verdict semantics.
    #[must_use]
    pub(crate) fn adjusted_alpha(&self) -> f64 {
        match self.correction {
            Correction::Holm | Correction::Bonferroni => {
                self.family_alpha / self.primary_cells as f64
            }
            Correction::None => self.family_alpha,
        }
    }
}

/// One evidence dimension named by the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Dimension {
    /// The frozen candidate survived data it was not tuned on.
    OutOfSample,
    /// The result holds across sequential folds.
    WalkForward,
    /// The result holds across instruments, regimes, and nearby parameters.
    Robustness,
    /// The worst loss and its duration are acceptable.
    Drawdown,
    /// The result is not driven by a few trades or one period.
    Consistency,
    /// The sample is large enough to support the claim.
    TradeCount,
    /// Capital exposure is within the envelope.
    Exposure,
    /// The worst outcomes are bounded and understood.
    TailRisk,
    /// The result survives higher fees and slippage.
    CostSensitivity,
    /// The result is stable around the chosen parameters.
    ParameterStability,
    /// The result is corrected for the number of trials.
    MultipleTesting,
    /// The input is complete and aligned.
    DataQuality,
    /// The rule beats the matched controls.
    BaselineControl,
    /// The same inputs reproduce the artifact.
    Reproducibility,
    /// The result fits the intended account rules.
    PropFirmLimits,
}

impl Dimension {
    /// Every declared dimension.
    pub(crate) const ALL: [Self; 15] = [
        Self::OutOfSample,
        Self::WalkForward,
        Self::Robustness,
        Self::Drawdown,
        Self::Consistency,
        Self::TradeCount,
        Self::Exposure,
        Self::TailRisk,
        Self::CostSensitivity,
        Self::ParameterStability,
        Self::MultipleTesting,
        Self::DataQuality,
        Self::BaselineControl,
        Self::Reproducibility,
        Self::PropFirmLimits,
    ];

    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::OutOfSample => "out_of_sample",
            Self::WalkForward => "walk_forward",
            Self::Robustness => "robustness",
            Self::Drawdown => "drawdown",
            Self::Consistency => "consistency",
            Self::TradeCount => "trade_count",
            Self::Exposure => "exposure",
            Self::TailRisk => "tail_risk",
            Self::CostSensitivity => "cost_sensitivity",
            Self::ParameterStability => "parameter_stability",
            Self::MultipleTesting => "multiple_testing",
            Self::DataQuality => "data_quality",
            Self::BaselineControl => "baseline_control",
            Self::Reproducibility => "reproducibility",
            Self::PropFirmLimits => "prop_firm_limits",
        }
    }

    /// Parses a stable wire code.
    #[must_use]
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|dimension| dimension.code() == code)
    }
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operator {
    /// Strictly greater than.
    Greater,
    /// Greater than or equal to.
    GreaterOrEqual,
    /// Strictly less than.
    Less,
    /// Less than or equal to.
    LessOrEqual,
    /// Exactly equal to.
    Equal,
    /// Not equal to.
    NotEqual,
}

impl Operator {
    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
        }
    }

    /// Parses a stable wire code.
    #[must_use]
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            ">" => Some(Self::Greater),
            ">=" => Some(Self::GreaterOrEqual),
            "<" => Some(Self::Less),
            "<=" => Some(Self::LessOrEqual),
            "==" => Some(Self::Equal),
            "!=" => Some(Self::NotEqual),
            _ => None,
        }
    }

    /// Compares one observed value against one threshold.
    #[must_use]
    pub(crate) fn evaluate(self, observed: f64, threshold: f64) -> bool {
        match self {
            Self::Greater => observed > threshold,
            Self::GreaterOrEqual => observed >= threshold,
            Self::Less => observed < threshold,
            Self::LessOrEqual => observed <= threshold,
            Self::Equal => observed == threshold,
            Self::NotEqual => observed != threshold,
        }
    }
}

/// An artifact a criterion can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EvidenceArtifact {
    /// The backtest manifest.
    Backtest,
    /// The portfolio metrics.
    Portfolio,
    /// The execution provenance.
    Execution,
    /// The model result.
    Model,
}

impl EvidenceArtifact {
    /// Every supported artifact.
    pub(crate) const ALL: [Self; 4] = [
        Self::Backtest,
        Self::Portfolio,
        Self::Execution,
        Self::Model,
    ];

    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Backtest => "backtest",
            Self::Portfolio => "portfolio",
            Self::Execution => "execution",
            Self::Model => "model",
        }
    }

    /// The required prefix of `evidence_field`.
    #[must_use]
    pub(crate) const fn field_prefix(self) -> &'static str {
        match self {
            Self::Backtest => "manifest.",
            Self::Portfolio => "metrics.",
            Self::Execution => "provenance.",
            Self::Model => "result.",
        }
    }

    /// Parses a stable wire code.
    #[must_use]
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|artifact| artifact.code() == code)
    }

    /// Strips the required prefix from a validated evidence field.
    #[must_use]
    pub(crate) fn field_path(self, field: &str) -> &str {
        field.strip_prefix(self.field_prefix()).unwrap_or(field)
    }
}

/// One human-owned criterion.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Criterion {
    /// The stable criterion id.
    pub(crate) id: String,
    /// The evidence dimension.
    pub(crate) dimension: Dimension,
    /// Whether the criterion changes the verdict.
    pub(crate) mandatory: bool,
    /// The human label for the measurement.
    pub(crate) metric: String,
    /// The comparison operator.
    pub(crate) operator: Operator,
    /// The comparison threshold.
    pub(crate) threshold: f64,
    /// The artifact the measurement comes from.
    pub(crate) evidence_artifact: EvidenceArtifact,
    /// The dotted field path inside the artifact.
    pub(crate) evidence_field: String,
}

/// A dimension the human declared not applicable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NotApplicable {
    /// The dimension.
    pub(crate) dimension: Dimension,
    /// Why the dimension does not apply.
    pub(crate) reason: String,
}

/// One validated qualification criteria file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Criteria {
    /// The human label for this protocol.
    pub(crate) id: String,
    /// The partitions the evidence must come from.
    pub(crate) required_partitions: Vec<Partition>,
    /// The sample floor the evidence must meet.
    pub(crate) minimum_trades: u64,
    /// The declared cost model.
    pub(crate) cost_model: CostModel,
    /// The pre-registered multiple-testing declaration.
    pub(crate) multiple_testing: MultipleTesting,
    /// The criteria, in file order.
    pub(crate) criteria: Vec<Criterion>,
    /// The dimensions the human declared not applicable.
    pub(crate) not_applicable: Vec<NotApplicable>,
}
