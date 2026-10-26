//! Presentation labels for the model's numeric codes.
//!
//! The model publishes numbers; the shell maps them to words. Every mapping is
//! fixed, so the same numeric output renders as the same text on every run.

use quantrule_model::dashboard::{
    AxisReadiness, BiasContinuity, BiasState, ChurnClass, Domain, MajorityClass, PaceDirection,
    Stage, TenureOriginKind, TransitionKind,
};

pub(crate) fn timeframe_label(seconds: u64) -> &'static str {
    match seconds {
        60 => "1m",
        300 => "5m",
        900 => "15m",
        3_600 => "1h",
        _ => "unknown",
    }
}

pub(crate) fn axis_label(code: u8) -> &'static str {
    match code {
        1 => "direction",
        2 => "momentum",
        3 => "structure",
        4 => "progression",
        _ => "unknown",
    }
}

pub(crate) fn direction_state_label(code: i8) -> &'static str {
    match code {
        -1 => "bearish",
        0 => "neutral",
        1 => "bullish",
        _ => "unknown",
    }
}

pub(crate) fn momentum_state_label(code: i8) -> &'static str {
    direction_state_label(code)
}

pub(crate) fn structure_label(code: i8) -> &'static str {
    match code {
        2 => "higher_high_higher_low",
        1 => "one_higher_relation",
        0 => "mixed_or_unchanged",
        -1 => "one_lower_relation",
        -2 => "lower_high_lower_low",
        _ => "unknown",
    }
}

pub(crate) fn stage_label(code: i8) -> &'static str {
    match code {
        0 => "against",
        1 => "under_25",
        2 => "25_to_50",
        3 => "50_to_75",
        4 => "late",
        5 => "terminal",
        _ => "unknown",
    }
}

pub(crate) fn context_label(code: u8) -> &'static str {
    match code {
        1 => "expanding",
        2 => "coiling",
        3 => "settling",
        4 => "high_range_low_participation",
        _ => "unknown",
    }
}

pub(crate) fn readiness_label(code: u8) -> &'static str {
    match code {
        0 => "unavailable",
        1 => "provisional",
        2 => "ready",
        _ => "unknown",
    }
}

pub(crate) fn reason_label(code: u8) -> &'static str {
    match code {
        1 => "insufficient_bars",
        2 => "insufficient_confirmed_pivots",
        3 => "invalid_denominator",
        4 => "no_target",
        5 => "target_unstable",
        6 => "origin_unconfirmed",
        _ => "unknown",
    }
}

pub(crate) fn event_type_label(code: u8) -> &'static str {
    match code {
        1 => "level",
        2 => "divergence",
        3 => "signal",
        _ => "unknown",
    }
}

pub(crate) fn event_subtype_label(type_code: u8, subtype_code: u8) -> &'static str {
    match (type_code, subtype_code) {
        (1, 1) => "test",
        (1, 2) => "cross",
        (1, 3) => "break",
        (2, 1) => "regular_bullish",
        (2, 2) => "regular_bearish",
        (2, 3) => "hidden_bullish",
        (2, 4) => "hidden_bearish",
        (3, 1) => "threshold_crossing",
        (3, 2) => "series_crossover",
        (3, 3) => "compact_state_change",
        _ => "unknown",
    }
}

pub(crate) fn source_label(code: u16) -> &'static str {
    match code {
        1 => "ema20",
        2 => "ema50",
        3 => "ema200",
        4 => "rsi14",
        5 => "atr14",
        6 => "rvol20",
        7 => "minor_pivots",
        8 => "major_pivots",
        9 => "direction_alignment",
        10 => "structure_relations",
        11 => "momentum_force",
        12 => "leg_progression",
        _ => "unknown",
    }
}

pub(crate) fn lifecycle_label(code: u8) -> &'static str {
    match code {
        0 => "forming",
        1 => "active",
        2 => "completed",
        3 => "invalidated",
        4 => "superseded",
        _ => "unknown",
    }
}

pub(crate) fn scale_label(code: u8) -> &'static str {
    match code {
        1 => "minor",
        2 => "major",
        _ => "unknown",
    }
}

pub(crate) fn leg_direction_label(code: i8) -> &'static str {
    match code {
        1 => "up",
        -1 => "down",
        _ => "unknown",
    }
}

pub(crate) fn target_basis_label(code: u8) -> &'static str {
    match code {
        1 => "confirmed_level",
        2 => "atr_fallback",
        _ => "unknown",
    }
}

/// The dashboard's six progression anchors, in text form.
pub(crate) fn dashboard_stage_label(stage: Stage) -> &'static str {
    match stage {
        Stage::Against => "against",
        Stage::Early => "early",
        Stage::EarlyMid => "early_mid",
        Stage::Mid => "mid",
        Stage::Late => "late",
        Stage::Terminal => "terminal",
    }
}

/// The dashboard's stable JSON code for a progression anchor.
pub(crate) fn dashboard_stage_code(stage: Stage) -> &'static str {
    match stage {
        Stage::Against => "Against",
        Stage::Early => "Early",
        Stage::EarlyMid => "EarlyMid",
        Stage::Mid => "Mid",
        Stage::Late => "Late",
        Stage::Terminal => "Terminal",
    }
}

/// The stable JSON code for a signed-value domain.
pub(crate) fn domain_code(domain: Domain) -> &'static str {
    match domain {
        Domain::DirectionalBias => "directional_bias",
        Domain::Acceleration => "acceleration",
        Domain::StructureRelation => "structure_relation",
    }
}

/// The stable JSON code for a visual readiness flag.
pub(crate) fn axis_readiness_code(readiness: AxisReadiness) -> &'static str {
    match readiness {
        AxisReadiness::Solid => "solid",
        AxisReadiness::Dashed => "dashed",
    }
}

pub(crate) fn bias_state_label(state: BiasState) -> &'static str {
    match state {
        BiasState::Bullish => "bullish",
        BiasState::Bearish => "bearish",
        BiasState::Neutral => "neutral",
        BiasState::Undefined => "undefined",
    }
}

pub(crate) fn bias_continuity_label(continuity: BiasContinuity) -> &'static str {
    match continuity {
        BiasContinuity::Continuous => "continuous",
        BiasContinuity::Interrupted => "interrupted",
        BiasContinuity::Undefined => "undefined",
    }
}

pub(crate) fn tenure_origin_label(kind: TenureOriginKind) -> &'static str {
    match kind {
        TenureOriginKind::None => "none",
        TenureOriginKind::FirstEstablishment => "first_establishment",
        TenureOriginKind::Reversal => "reversal",
        TenureOriginKind::NewOriginAfterUnavailable => "new_origin_after_unavailable",
    }
}

pub(crate) fn pace_direction_label(direction: PaceDirection) -> &'static str {
    match direction {
        PaceDirection::WithLeg => "with_leg",
        PaceDirection::AgainstLeg => "against_leg",
        PaceDirection::Flat => "flat",
    }
}

pub(crate) fn majority_label(class: MajorityClass) -> &'static str {
    match class {
        MajorityClass::Bullish => "bullish",
        MajorityClass::Bearish => "bearish",
        MajorityClass::Neutral => "neutral",
    }
}

pub(crate) fn churn_class_label(class: ChurnClass) -> &'static str {
    match class {
        ChurnClass::Building => "Building",
        ChurnClass::Holding => "Holding",
        ChurnClass::Unstable => "Unstable",
    }
}

/// The stable name of a current Structure reference kind.
pub(crate) fn reference_kind_label(code: u8) -> &'static str {
    match code {
        1 => "ema20",
        2 => "ema50",
        3 => "ema200",
        4 => "minor_high",
        5 => "minor_low",
        6 => "major_high",
        7 => "major_low",
        _ => "unknown",
    }
}

/// The stable JSON code for a transition kind.
pub(crate) fn transition_kind_code(kind: TransitionKind) -> &'static str {
    match kind {
        TransitionKind::CompactState => "compact_state",
        TransitionKind::ProgressionStage => "progression_stage",
    }
}
