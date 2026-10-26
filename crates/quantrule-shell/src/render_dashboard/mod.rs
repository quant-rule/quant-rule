//! Rendering for the four-axis dashboard.
//!
//! The shell maps the model's numeric [`DashboardState`] to the fixed terminal
//! text. The numeric JSON envelope lives in [`json`]; both renderers copy
//! published values and compute nothing.
//!
//! The text geometry is fixed by the command reference and locked by a golden
//! test. Position is the dot's slot in an eleven-slot track, size is the
//! three-step radius glyph, and readiness is the solid or hollow outline. An
//! undefined position has no dot and prints its reason code instead.

use std::io::{self, Write};

use quantrule_data::format_utc_timestamp;
use quantrule_model::dashboard::{
    AxisAgreement, AxisDetail, AxisState, AxisValue, CrossTfAgreement, CrossTfChurn,
    DashboardState, DatedReading, DirectionDetail, LevelReading, MomentumDetail, ProgressionDetail,
    StructureDetail, TimeframeState, TransitionReading,
};
use quantrule_model::readings::roles::AxisCode;

use crate::labels;

mod json;

pub(crate) use json::write_json;

/// The dashboard view-contract version printed in the header and JSON.
const VIEW_CONTRACT: &str = "2026-10-25";
/// The number of slots in one position track.
const TRACK_SLOTS: usize = 11;
/// The fixed width of one axis cell.
const CELL_WIDTH: usize = 27;
/// The gap between two axis cells.
const CELL_GAP: &str = "  ";

/// Writes the deterministic text dashboard.
///
/// The output contains no clock, path, or environment value. The same state and
/// window print the same bytes on every run and through both entry points.
pub(crate) fn write_text(
    state: &DashboardState,
    window: &str,
    writer: &mut impl Write,
) -> io::Result<()> {
    writeln!(
        writer,
        "market-dashboard  window={window}  ladder={}  model={}  view={VIEW_CONTRACT}",
        timeframe_codes(&state.ladder),
        env!("CARGO_PKG_VERSION")
    )?;
    write_as_of(state, writer)?;
    write_agreement(&state.cross_tf_state.agreement, writer)?;
    write_churn(&state.cross_tf_state.churn, writer)?;
    writeln!(writer)?;

    write_track_header(writer)?;
    for timeframe in &state.timeframes {
        write_track_row(timeframe, writer)?;
    }

    if state.timeframes.iter().any(|timeframe| timeframe.present) {
        writeln!(writer)?;
        writeln!(writer, "diagnostics")?;
        for timeframe in state
            .timeframes
            .iter()
            .filter(|timeframe| timeframe.present)
        {
            write_diagnostics(timeframe, writer)?;
        }

        writeln!(writer)?;
        writeln!(writer, "evidence")?;
        for timeframe in state
            .timeframes
            .iter()
            .filter(|timeframe| timeframe.present)
        {
            write_evidence(timeframe, writer)?;
        }

        writeln!(writer)?;
        writeln!(writer, "readings")?;
        for timeframe in state
            .timeframes
            .iter()
            .filter(|timeframe| timeframe.present)
        {
            write_readings(timeframe, writer)?;
        }
    }

    Ok(())
}

fn write_as_of(state: &DashboardState, writer: &mut impl Write) -> io::Result<()> {
    let mut parts = Vec::with_capacity(state.timeframes.len());
    for timeframe in &state.timeframes {
        let label = labels::timeframe_label(timeframe.timeframe_secs);
        match timeframe.latest_open_time {
            Some(time) => parts.push(format!("{label} {}", format_utc_timestamp(time))),
            None => parts.push(format!("{label} — no closed bar")),
        }
    }
    writeln!(writer, "as-of  {}", parts.join(" | "))
}

fn write_agreement(agreement: &CrossTfAgreement, writer: &mut impl Write) -> io::Result<()> {
    let details = agreement
        .axes
        .iter()
        .map(agreement_axis_text)
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(
        writer,
        "cross-TF agreement  {}/{} axes have a strict majority ({details}) · coverage {}/{}",
        agreement.agreeing_axes,
        agreement.of,
        agreement.coverage.present,
        agreement.coverage.configured
    )
}

fn agreement_axis_text(axis: &AxisAgreement) -> String {
    let label = labels::axis_label(axis.axis.code());
    match axis.majority {
        Some(class) => format!(
            "{label} {} {}/{}",
            labels::majority_label(class),
            majority_count(axis, class),
            axis.valid_timeframes
        ),
        None if axis.insufficient_coverage => format!("{label} insufficient coverage"),
        None => format!("{label} no majority"),
    }
}

fn majority_count(axis: &AxisAgreement, class: quantrule_model::dashboard::MajorityClass) -> u8 {
    match class {
        quantrule_model::dashboard::MajorityClass::Bullish => axis.bullish_count,
        quantrule_model::dashboard::MajorityClass::Bearish => axis.bearish_count,
        quantrule_model::dashboard::MajorityClass::Neutral => axis.neutral_count,
    }
}

fn write_churn(churn: &CrossTfChurn, writer: &mut impl Write) -> io::Result<()> {
    let sources = if churn.sources.is_empty() {
        "—".to_owned()
    } else {
        churn
            .sources
            .iter()
            .map(|source| {
                format!(
                    "{} {}",
                    labels::timeframe_label(source.timeframe_secs),
                    labels::axis_label(source.axis.code())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let history = if churn.history_complete {
        "complete"
    } else {
        "incomplete"
    };
    writeln!(
        writer,
        "churn  max {} state changes ({sources}) · threshold {} · history {}/{} ({history}) · class {}",
        churn.max_changes_50,
        churn.threshold,
        churn.coverage.present,
        churn.coverage.configured,
        labels::churn_class_label(churn.churn_class)
    )
}

fn write_track_header(writer: &mut impl Write) -> io::Result<()> {
    let labels = [
        "DIRECTION -1..+1",
        "MOMENTUM decel..accel",
        "STRUCTURE bear..bull",
        "PROGRESSION against..terminal",
    ];
    let line = labels
        .iter()
        .map(|label| format!("{label:<CELL_WIDTH$}"))
        .collect::<Vec<_>>()
        .join(CELL_GAP);
    writeln!(writer, "      {}", line.trim_end())?;
    writeln!(
        writer,
        "legend  ● position · small • medium ● large  ■ solid □ dashed  — unavailable (r<code>)"
    )
}

fn write_track_row(timeframe: &TimeframeState, writer: &mut impl Write) -> io::Result<()> {
    let label = labels::timeframe_label(timeframe.timeframe_secs);
    if !timeframe.present {
        return writeln!(writer, "  {label:<4}— no closed bar");
    }
    let cells = timeframe
        .axes
        .iter()
        .map(|axis| format!("{:<CELL_WIDTH$}", track_cell(axis)))
        .collect::<Vec<_>>()
        .join(CELL_GAP);
    writeln!(writer, "  {label:<4}{}", cells.trim_end())
}

fn track_cell(axis: &AxisState) -> String {
    let track = position_track(&axis.position);
    let (percent, size) = match axis.size {
        AxisValue::Rank { pct } if pct.is_finite() => {
            let pct = pct.clamp(0.0, 100.0);
            (format!("{:>3.0}%", pct.round()), size_glyph(pct))
        }
        _ => ("   —".to_owned(), '—'),
    };
    let readiness = match axis.readiness {
        quantrule_model::dashboard::AxisReadiness::Solid => '■',
        quantrule_model::dashboard::AxisReadiness::Dashed => '□',
    };
    format!("{track} {percent} {size}{readiness}")
}

fn position_track(position: &AxisValue) -> String {
    let slot = match position {
        AxisValue::Signed { value, .. } if value.is_finite() => Some(signed_slot(*value)),
        AxisValue::Ordinal { stage } => Some(stage_slot(*stage)),
        _ => None,
    };
    match slot {
        Some(slot) => {
            let mut track = ['─'; TRACK_SLOTS];
            track[slot] = '●';
            track.iter().collect()
        }
        None => {
            let mut text = String::from("—");
            if let AxisValue::Undefined { reason } = position {
                text.push_str(&format!(" r{}", reason.code()));
            }
            format!("{text:<TRACK_SLOTS$}")
        }
    }
}

fn signed_slot(value: f64) -> usize {
    let normalized = (value.clamp(-1.0, 1.0) + 1.0) / 2.0;
    ((normalized * 10.0).round() as usize).min(TRACK_SLOTS - 1)
}

fn stage_slot(stage: quantrule_model::dashboard::Stage) -> usize {
    (stage.code() as usize * 2).min(TRACK_SLOTS - 1)
}

fn size_glyph(pct: f64) -> char {
    if pct < 100.0 / 3.0 {
        '·'
    } else if pct < 200.0 / 3.0 {
        '•'
    } else {
        '●'
    }
}

fn write_diagnostics(timeframe: &TimeframeState, writer: &mut impl Write) -> io::Result<()> {
    let label = labels::timeframe_label(timeframe.timeframe_secs);
    let consensus = &timeframe.derived.timeframe_consensus;
    let weakest = consensus
        .weakest_directional_axis_code
        .map(labels::axis_label)
        .unwrap_or("—");
    let context = timeframe
        .diagnostics
        .context_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "—".to_owned());
    writeln!(
        writer,
        "  diag      {label:<4}atr={} rvol={} context={context} consensus={}/{}/{} weakest={weakest}",
        option_number(timeframe.diagnostics.atr_ratio),
        option_number(timeframe.diagnostics.rvol_ratio),
        consensus.bullish_axis_count,
        consensus.bearish_axis_count,
        consensus.neutral_axis_count
    )
}

fn write_evidence(timeframe: &TimeframeState, writer: &mut impl Write) -> io::Result<()> {
    let label = labels::timeframe_label(timeframe.timeframe_secs);
    for axis in &timeframe.axes {
        match &axis.detail {
            AxisDetail::Direction(detail) => {
                writeln!(
                    writer,
                    "  evidence  {label:<4}{:<8}{}",
                    "DIR",
                    direction_evidence(axis, detail)
                )?;
            }
            AxisDetail::Momentum(detail) => {
                writeln!(
                    writer,
                    "  evidence  {label:<4}{:<8}{}",
                    "MOM",
                    momentum_evidence(detail)
                )?;
            }
            AxisDetail::Structure(detail) => {
                writeln!(
                    writer,
                    "  evidence  {label:<4}{:<8}{}",
                    "STRUCT",
                    structure_evidence(detail)
                )?;
                writeln!(
                    writer,
                    "  levels    {label:<4}{}",
                    level_references_text(&detail.level_references)
                )?;
            }
            AxisDetail::Progression(detail) => {
                writeln!(
                    writer,
                    "  evidence  {label:<4}{:<8}{}",
                    "PROG",
                    progression_evidence(axis, detail)
                )?;
            }
        }
    }
    Ok(())
}

fn direction_evidence(axis: &AxisState, detail: &DirectionDetail) -> String {
    let tenure = match detail.bias_tenure_bars {
        Some(bars) => match axis.size.as_rank() {
            Some(pct) => format!("{bars} ({})", percent(pct)),
            None => bars.to_string(),
        },
        None => "—".to_owned(),
    };
    // A neutral pause is not an active bias; the contract requires the prior
    // tenure to be named so a large radius cannot be read as a live direction.
    let tenure_label = match detail.bias_state {
        quantrule_model::dashboard::BiasState::Neutral => "prior bias tenure",
        _ => "tenure",
    };
    format!(
        "{} · {tenure_label} {tenure} · continuity {} · origin {}@{} · magnitude {} · age {} · churn {}",
        labels::bias_state_label(detail.bias_state),
        labels::bias_continuity_label(detail.bias_continuity),
        labels::tenure_origin_label(detail.tenure_origin_kind),
        optional(detail.tenure_origin_bar),
        option_percent(detail.magnitude_pct),
        detail.compact_state_age_bars,
        detail.compact_state_changes_50
    )
}

fn momentum_evidence(detail: &MomentumDetail) -> String {
    format!(
        "force {} · acceleration {} · intensification {} · age {} · churn {}",
        option_signed_number(detail.force),
        option_signed_number(detail.acceleration),
        option_signed_number(detail.intensification),
        detail.compact_state_age_bars,
        detail.compact_state_changes_50
    )
}

fn structure_evidence(detail: &StructureDetail) -> String {
    format!(
        "score {} · HH {} HL {} · {} · d20 {} d50 {} d200 {} · x20 {} x50 {} x200 {} · t20 {} t50 {} t200 {} · levels {} · activity {}/{} ({}/bar) · age {} · churn {}",
        option_signed_number(detail.score),
        option_signed_code(detail.high_relation_code),
        option_signed_code(detail.low_relation_code),
        labels::scale_label(detail.scale_code),
        option_signed_number(detail.ema_distance_atr_20),
        option_signed_number(detail.ema_distance_atr_50),
        option_signed_number(detail.ema_distance_atr_200),
        option_signed_code(detail.ema_cross_code_20),
        option_signed_code(detail.ema_cross_code_50),
        option_signed_code(detail.ema_cross_code_200),
        option_signed_code(detail.ema_retest_code_20),
        option_signed_code(detail.ema_retest_code_50),
        option_signed_code(detail.ema_retest_code_200),
        detail.confirmed_level_count,
        detail.activity_count,
        detail.activity_window,
        option_number(detail.activity_per_bar),
        detail.compact_state_age_bars,
        detail.compact_state_changes_50
    )
}

fn level_references_text(references: &[LevelReading]) -> String {
    if references.is_empty() {
        return "—".to_owned();
    }
    references
        .iter()
        .map(|reference| {
            format!(
                "{} {:.5}@{}",
                labels::reference_kind_label(reference.reference_kind_code),
                reference.price,
                reference.available_bar
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn progression_evidence(axis: &AxisState, detail: &ProgressionDetail) -> String {
    let stage = stage_text(detail.stage_code);
    let leg_direction = detail
        .leg_direction
        .map(labels::leg_direction_label)
        .unwrap_or("—");
    let leg_scale = detail
        .leg_scale_code
        .map(labels::scale_label)
        .unwrap_or("—");
    let lifecycle = detail
        .lifecycle_code
        .map(labels::lifecycle_label)
        .unwrap_or("—");
    let target_version = optional(detail.target_version);
    let target_stability = match detail.target_stable {
        Some(true) => "stable",
        Some(false) => "unstable",
        None => "—",
    };
    let pace_direction = detail
        .recent_pace_direction
        .map(labels::pace_direction_label)
        .unwrap_or("—");
    format!(
        "stage {stage} · progress {} · leg {leg_direction} {leg_scale} · lifecycle {lifecycle} · target v{target_version} {target_stability} · pace {} {pace_direction} · speed {} · avg net {} · disp {} · origin {}@{} · frozen {}",
        option_signed_number(detail.progress),
        option_signed_number(detail.recent_pace_signed_atr_per_bar),
        option_percent(axis.size.as_rank()),
        option_number(detail.average_net_displacement_atr_per_bar),
        option_percent(detail.displacement_pct),
        optional(detail.origin_bar),
        optional(detail.origin_available_bar),
        detail.recent_pace_frozen_terminal
    )
}

fn stage_text(code: Option<i8>) -> String {
    code.and_then(quantrule_model::dashboard::Stage::from_code)
        .map(labels::dashboard_stage_label)
        .unwrap_or("—")
        .to_owned()
}

fn write_readings(timeframe: &TimeframeState, writer: &mut impl Write) -> io::Result<()> {
    let label = labels::timeframe_label(timeframe.timeframe_secs);
    let segments = timeframe
        .axes
        .iter()
        .map(|axis| {
            let reading = axis
                .readings
                .last()
                .map(|reading| reading_text(axis.axis, reading))
                .unwrap_or_else(|| "—".to_owned());
            format!("{} {reading}", short_axis_label(axis.axis))
        })
        .collect::<Vec<_>>()
        .join(" | ");
    writeln!(writer, "  reading   {label:<4}{segments}")
}

fn reading_text(axis: AxisCode, reading: &DatedReading) -> String {
    match reading {
        DatedReading::Event(event) => format!(
            "event {} {} occ={} avail={} dir={} ref={} primary={} reference={}",
            labels::event_type_label(event.event_type_code),
            labels::event_subtype_label(event.event_type_code, event.event_subtype_code),
            event.occurred_bar,
            event.available_bar,
            signed_code(event.direction_code),
            optional(event.reference_code),
            option_number(event.primary_value),
            option_number(event.reference_value)
        ),
        DatedReading::Transition(transition) => format!(
            "transition {} {}->{} occ={} avail={}",
            transition_kind_text(transition),
            transition_code_text(axis, transition, transition.from_code),
            transition_code_text(axis, transition, transition.to_code),
            transition.occurred_bar,
            transition.available_bar
        ),
    }
}

fn transition_kind_text(transition: &TransitionReading) -> &'static str {
    match transition.kind {
        quantrule_model::dashboard::TransitionKind::CompactState => "compact",
        quantrule_model::dashboard::TransitionKind::ProgressionStage => "stage",
    }
}

fn transition_code_text(
    axis: AxisCode,
    transition: &TransitionReading,
    code: Option<i8>,
) -> String {
    let Some(code) = code else {
        return "—".to_owned();
    };
    match transition.kind {
        quantrule_model::dashboard::TransitionKind::CompactState => match axis {
            AxisCode::Direction => labels::direction_state_label(code).to_owned(),
            AxisCode::Momentum => labels::momentum_state_label(code).to_owned(),
            AxisCode::Structure => labels::structure_label(code).to_owned(),
            AxisCode::Progression => stage_text(Some(code)),
        },
        quantrule_model::dashboard::TransitionKind::ProgressionStage => stage_text(Some(code)),
    }
}

fn short_axis_label(axis: AxisCode) -> &'static str {
    match axis {
        AxisCode::Direction => "DIR",
        AxisCode::Momentum => "MOM",
        AxisCode::Structure => "STRUCT",
        AxisCode::Progression => "PROG",
    }
}

fn timeframe_codes(ladder: &[u64]) -> String {
    ladder
        .iter()
        .map(|seconds| labels::timeframe_label(*seconds))
        .collect::<Vec<_>>()
        .join(",")
}

fn percent(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    format!("{:.0}%", value.clamp(0.0, 100.0).round())
}

fn option_percent(value: Option<f64>) -> String {
    value.map(percent).unwrap_or_else(|| "—".to_owned())
}

fn number(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value == 0.0 {
        "0.00".to_owned()
    } else {
        format!("{value:.2}")
    }
}

fn option_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_else(|| "—".to_owned())
}

fn signed_number(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value > 0.0 {
        format!("+{}", number(value))
    } else {
        number(value)
    }
}

fn option_signed_number(value: Option<f64>) -> String {
    value.map(signed_number).unwrap_or_else(|| "—".to_owned())
}

fn signed_code(code: i8) -> String {
    match code {
        value if value > 0 => format!("+{value}"),
        value => value.to_string(),
    }
}

fn option_signed_code(code: Option<i8>) -> String {
    code.map(signed_code).unwrap_or_else(|| "—".to_owned())
}

fn optional<T: ToString>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_model::dashboard::{
        AxisReadiness, BiasContinuity, BiasState, DashboardConfig, Domain, Stage, TenureOriginKind,
    };
    use quantrule_model::run::ModelRun;
    use quantrule_model::state::MarketStateConfig;
    use serde_json::{Value, json};

    fn empty_state() -> DashboardState {
        let run = ModelRun::compute(&[], &MarketStateConfig::default());
        DashboardState::from_run(&run, &[60, 300, 900, 3_600], &DashboardConfig::default())
    }

    #[test]
    fn signed_positions_use_all_eleven_slots() {
        assert_eq!(signed_slot(-1.0), 0);
        assert_eq!(signed_slot(-0.8), 1);
        assert_eq!(signed_slot(0.0), 5);
        assert_eq!(signed_slot(0.8), 9);
        assert_eq!(signed_slot(1.0), 10);
        assert_eq!(signed_slot(-4.0), 0);
        assert_eq!(signed_slot(4.0), 10);
    }

    #[test]
    fn stage_positions_are_evenly_spaced() {
        for (code, slot) in [(0, 0), (1, 2), (2, 4), (3, 6), (4, 8), (5, 10)] {
            let stage = quantrule_model::dashboard::Stage::from_code(code);
            assert_eq!(stage.map(stage_slot), Some(slot), "stage code {code}");
        }
        assert_eq!(quantrule_model::dashboard::Stage::from_code(-1), None);
        assert_eq!(stage_slot(Stage::Terminal), 10);
    }

    #[test]
    fn size_has_three_steps_and_keeps_a_valid_zero() {
        assert_eq!(size_glyph(0.0), '·');
        assert_eq!(size_glyph(33.0), '·');
        assert_eq!(size_glyph(34.0), '•');
        assert_eq!(size_glyph(66.0), '•');
        assert_eq!(size_glyph(67.0), '●');
        assert_eq!(size_glyph(100.0), '●');
    }

    #[test]
    fn undefined_position_has_no_dot_and_prints_the_reason_code() {
        let value =
            AxisValue::undefined(quantrule_model::kernel::UnavailableReason::InsufficientBars);
        assert_eq!(position_track(&value), "— r1       ");
    }

    #[test]
    fn a_neutral_direction_names_the_prior_bias_tenure() {
        let axis = AxisState {
            axis: AxisCode::Direction,
            position: AxisValue::signed(0.0, Domain::DirectionalBias),
            size: AxisValue::rank(80.0),
            readiness: AxisReadiness::Solid,
            readiness_code: 2,
            readings: Vec::new(),
            detail: AxisDetail::Direction(DirectionDetail {
                bias_state: BiasState::Neutral,
                bias_continuity: BiasContinuity::Continuous,
                bias_tenure_bars: Some(46),
                tenure_origin_kind: TenureOriginKind::FirstEstablishment,
                tenure_origin_bar: Some(0),
                magnitude_pct: Some(50.0),
                compact_state_age_bars: 2,
                compact_state_changes_50: 1,
            }),
        };
        let AxisDetail::Direction(detail) = &axis.detail else {
            panic!("the test axis is Direction");
        };
        assert!(
            direction_evidence(&axis, detail).starts_with("neutral · prior bias tenure 46 (80%)")
        );
    }

    #[test]
    fn text_is_byte_identical_across_repeated_runs() {
        let state = empty_state();
        let mut first = Vec::new();
        let mut second = Vec::new();
        write_text(&state, "full", &mut first).expect("text writes");
        write_text(&state, "full", &mut second).expect("text writes");
        assert_eq!(first, second);
    }

    #[test]
    fn absent_rows_are_printed_and_not_zero() {
        let state = empty_state();
        let mut text = Vec::new();
        write_text(&state, "full", &mut text).expect("text writes");
        let text = String::from_utf8(text).expect("text is UTF-8");
        assert_eq!(text.matches("— no closed bar").count(), 8);
        assert!(!text.contains(" 0%"));
    }

    #[test]
    fn json_is_one_object_with_the_three_part_envelope() {
        let state = empty_state();
        let mut json = Vec::new();
        write_json(&state, &mut json).expect("JSON writes");
        let text = String::from_utf8(json).expect("JSON is UTF-8");
        assert!(text.ends_with('\n'));
        let value: Value = serde_json::from_str(&text).expect("JSON parses");
        assert_eq!(value["view_contract"], VIEW_CONTRACT);
        assert_eq!(
            value["dashboard_state"]["ladder"],
            json!([60, 300, 900, 3600])
        );
        assert_eq!(
            value["dashboard_state"]["timeframes"]
                .as_array()
                .map(Vec::len),
            Some(4)
        );
    }

    #[test]
    fn text_has_no_input_path() {
        let state = empty_state();
        let mut text = Vec::new();
        write_text(&state, "latest", &mut text).expect("text writes");
        let text = String::from_utf8(text).expect("text is UTF-8");
        assert!(!text.contains("path/to"));
    }
}
