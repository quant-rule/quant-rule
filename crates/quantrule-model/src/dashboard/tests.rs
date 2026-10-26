use super::{
    AxisAgreement, AxisDetail, AxisReadiness, AxisValue, BiasContinuity, BiasState, ChurnClass,
    ChurnSource, Coverage, CrossTfAgreement, CrossTfChurn, DEFAULT_CHURN_THRESHOLD,
    DEFAULT_MIN_CROSS_TF_MEMBERS, DashboardConfig, DatedReading, Domain, EventReading,
    LevelReading, MajorityClass, PaceDirection, RECENT_PACE_WINDOW_BARS, Stage, TenureOriginKind,
    TransitionKind, TransitionReading,
};
use crate::kernel::{Readiness, UnavailableReason};
use crate::readings::roles::AxisCode;

#[test]
fn axis_values_keep_their_variant_and_domain() {
    let signed = AxisValue::signed(-0.42, Domain::DirectionalBias);
    assert_eq!(signed.as_signed(), Some((-0.42, Domain::DirectionalBias)));
    assert_eq!(signed.as_rank(), None);
    assert_eq!(signed.as_stage(), None);
    assert!(!signed.is_undefined());

    let rank = AxisValue::rank(71.5);
    assert_eq!(rank.as_rank(), Some(71.5));
    assert_eq!(rank.as_signed(), None);

    let ordinal = AxisValue::ordinal(Stage::Late);
    assert_eq!(ordinal.as_stage(), Some(Stage::Late));
    assert_eq!(ordinal.as_rank(), None);

    let absent = AxisValue::undefined(UnavailableReason::OriginUnconfirmed);
    assert!(absent.is_undefined());
    assert_eq!(absent.as_signed(), None);
    assert_eq!(absent.as_rank(), None);
    assert_eq!(absent.as_stage(), None);
}

#[test]
fn domains_stay_distinct() {
    let direction = AxisValue::signed(0.4, Domain::DirectionalBias);
    let acceleration = AxisValue::signed(0.4, Domain::Acceleration);
    let structure = AxisValue::signed(0.4, Domain::StructureRelation);

    assert_ne!(direction, acceleration);
    assert_ne!(direction, structure);
    assert_ne!(acceleration, structure);
}

#[test]
fn stages_are_the_six_contract_codes() {
    let codes: Vec<i8> = Stage::ALL.iter().map(|stage| stage.code()).collect();
    assert_eq!(codes, vec![0, 1, 2, 3, 4, 5]);
    for stage in Stage::ALL {
        assert_eq!(Stage::from_code(stage.code()), Some(stage));
    }
    assert_eq!(Stage::from_code(-1), None);
    assert_eq!(Stage::from_code(6), None);
}

#[test]
fn readiness_mapping_is_mechanical() {
    assert_eq!(AxisReadiness::from(Readiness::Ready), AxisReadiness::Solid);
    assert_eq!(
        AxisReadiness::from(Readiness::Provisional),
        AxisReadiness::Dashed
    );
    assert_eq!(
        AxisReadiness::from(Readiness::Unavailable),
        AxisReadiness::Dashed
    );
    assert_eq!(AxisReadiness::Solid.code(), 0);
    assert_eq!(AxisReadiness::Dashed.code(), 1);
}

#[test]
fn config_defaults_are_the_contract_values() {
    let config = DashboardConfig::default();
    assert_eq!(config.churn_threshold, DEFAULT_CHURN_THRESHOLD);
    assert_eq!(config.min_cross_tf_members, DEFAULT_MIN_CROSS_TF_MEMBERS);
    assert_eq!(config.churn_threshold, 6);
    assert_eq!(config.min_cross_tf_members, 2);
    assert_eq!(RECENT_PACE_WINDOW_BARS, 3);

    let explicit = DashboardConfig::new(9, 3);
    assert_eq!(explicit.churn_threshold, 9);
    assert_eq!(explicit.min_cross_tf_members, 3);
}

#[test]
fn level_and_dated_readings_carry_their_codes() {
    let level = LevelReading {
        reference_kind_code: 1,
        price: 64_000.0,
        available_bar: 42,
    };
    assert_eq!(level.reference_kind_code, 1);
    assert_eq!(level.price, 64_000.0);
    assert_eq!(level.available_bar, 42);

    let event = EventReading {
        event_type_code: 3,
        event_subtype_code: 1,
        source_code: 4,
        occurred_bar: 40,
        available_bar: 41,
        direction_code: 1,
        reference_code: None,
        primary_value: Some(72.0),
        reference_value: Some(70.0),
    };
    let dated = DatedReading::Event(event);
    match dated {
        DatedReading::Event(reading) => {
            assert_eq!(reading.event_type_code, 3);
            assert_eq!(reading.available_bar, 41);
        }
        DatedReading::Transition(_) => panic!("expected an event reading"),
    }

    let transition = TransitionReading {
        axis: AxisCode::Direction,
        kind: TransitionKind::CompactState,
        from_code: Some(0),
        to_code: Some(-1),
        occurred_bar: 50,
        available_bar: 50,
    };
    let dated = DatedReading::Transition(transition);
    match dated {
        DatedReading::Transition(reading) => {
            assert_eq!(reading.axis, AxisCode::Direction);
            assert_eq!(reading.kind, TransitionKind::CompactState);
            assert_eq!(reading.to_code, Some(-1));
        }
        DatedReading::Event(_) => panic!("expected a transition reading"),
    }
}

#[test]
fn direction_detail_keeps_neutral_and_continuity_explicit() {
    let detail = AxisDetail::Direction(super::DirectionDetail {
        bias_state: BiasState::Neutral,
        bias_continuity: BiasContinuity::Interrupted,
        bias_tenure_bars: Some(46),
        tenure_origin_kind: TenureOriginKind::NewOriginAfterUnavailable,
        tenure_origin_bar: Some(12),
        magnitude_pct: Some(71.0),
        compact_state_age_bars: 2,
        compact_state_changes_50: 3,
    });
    match detail {
        AxisDetail::Direction(direction) => {
            assert_eq!(direction.bias_state, BiasState::Neutral);
            assert_eq!(direction.bias_continuity, BiasContinuity::Interrupted);
            assert_eq!(direction.bias_tenure_bars, Some(46));
            assert_eq!(
                direction.tenure_origin_kind,
                TenureOriginKind::NewOriginAfterUnavailable
            );
        }
        _ => panic!("expected direction detail"),
    }
}

#[test]
fn momentum_detail_keeps_force_and_acceleration() {
    let detail = AxisDetail::Momentum(super::MomentumDetail {
        force: Some(-0.35),
        acceleration: Some(0.8),
        intensification: Some(0.12),
        compact_state_age_bars: 4,
        compact_state_changes_50: 2,
    });
    match detail {
        AxisDetail::Momentum(momentum) => {
            assert_eq!(momentum.force, Some(-0.35));
            assert_eq!(momentum.acceleration, Some(0.8));
            assert_eq!(momentum.compact_state_changes_50, 2);
        }
        _ => panic!("expected momentum detail"),
    }
}

#[test]
fn structure_detail_keeps_levels_and_activity_separate_from_the_score() {
    let detail = AxisDetail::Structure(super::StructureDetail {
        score: Some(0.5),
        high_relation_code: Some(1),
        low_relation_code: Some(0),
        scale_code: 2,
        ema_distance_atr_20: Some(0.8),
        ema_distance_atr_50: Some(1.1),
        ema_distance_atr_200: Some(1.4),
        ema_cross_code_20: Some(1),
        ema_cross_code_50: Some(0),
        ema_cross_code_200: None,
        ema_retest_code_20: Some(1),
        ema_retest_code_50: None,
        ema_retest_code_200: None,
        level_references: vec![LevelReading {
            reference_kind_code: 1,
            price: 64_000.0,
            available_bar: 42,
        }],
        confirmed_level_count: 4,
        activity_count: 3,
        activity_window: 5,
        activity_per_bar: Some(0.6),
        compact_state_age_bars: 1,
        compact_state_changes_50: 2,
    });
    match detail {
        AxisDetail::Structure(structure) => {
            assert_eq!(structure.score, Some(0.5));
            assert_eq!(structure.high_relation_code, Some(1));
            assert_eq!(structure.low_relation_code, Some(0));
            assert_eq!(structure.scale_code, 2);
            assert_eq!(structure.level_references.len(), 1);
            assert_eq!(structure.confirmed_level_count, 4);
            assert_eq!(structure.activity_count, 3);
            assert_eq!(structure.activity_window, 5);
            assert_eq!(structure.activity_per_bar, Some(0.6));
        }
        _ => panic!("expected structure detail"),
    }
}

#[test]
fn progression_detail_keeps_signed_pace_and_terminal_freeze_separate() {
    let detail = AxisDetail::Progression(super::ProgressionDetail {
        progress: Some(0.84),
        stage_code: Some(4),
        leg_direction: Some(1),
        leg_scale_code: Some(2),
        lifecycle_code: Some(1),
        target_version: Some(2),
        target_stable: Some(true),
        origin_bar: Some(10),
        origin_available_bar: Some(12),
        displacement_pct: Some(72.0),
        recent_pace_signed_atr_per_bar: Some(-0.12),
        recent_pace_direction: Some(PaceDirection::AgainstLeg),
        recent_pace_window_bars: RECENT_PACE_WINDOW_BARS,
        average_net_displacement_atr_per_bar: Some(0.08),
        recent_pace_frozen_terminal: true,
    });
    match detail {
        AxisDetail::Progression(progression) => {
            assert_eq!(progression.recent_pace_signed_atr_per_bar, Some(-0.12));
            assert_eq!(
                progression.recent_pace_direction,
                Some(PaceDirection::AgainstLeg)
            );
            assert_eq!(progression.recent_pace_window_bars, 3);
            assert!(progression.recent_pace_frozen_terminal);
        }
        _ => panic!("expected progression detail"),
    }
}

#[test]
fn dashboard_state_carries_the_ladder_and_the_cross_timeframe_state() {
    let coverage = Coverage {
        present: 0,
        configured: 4,
    };
    let state = super::DashboardState {
        ladder: vec![60, 300, 900, 3_600],
        timeframes: Vec::new(),
        cross_tf_state: super::CrossTfState {
            configured_timeframes: vec![60, 300, 900, 3_600],
            present_timeframes: Vec::new(),
            aggregates: crate::aggregate::CrossTimeframeAggregates::compute(&[]),
            agreement: CrossTfAgreement {
                of: 3,
                agreeing_axes: 0,
                axes: Vec::new(),
                coverage,
                min_valid_timeframes: 2,
            },
            churn: CrossTfChurn {
                max_changes_50: 0,
                sources: Vec::new(),
                threshold: 6,
                window: 50,
                history_complete: false,
                churn_class: ChurnClass::Building,
                coverage,
            },
            progression_leg_direction: Vec::new(),
        },
    };

    assert_eq!(state.ladder, vec![60, 300, 900, 3_600]);
    assert!(state.timeframes.is_empty());
    assert!(state.cross_tf_state.present_timeframes.is_empty());
    assert!(state.cross_tf_state.aggregates.ladder.is_empty());
    assert_eq!(state.cross_tf_state.agreement.of, 3);
    assert_eq!(state.cross_tf_state.churn.churn_class, ChurnClass::Building);
}

#[test]
fn agreement_and_churn_types_express_counts_not_scores() {
    let coverage = Coverage {
        present: 4,
        configured: 4,
    };
    let agreement = CrossTfAgreement {
        of: 3,
        agreeing_axes: 2,
        axes: vec![AxisAgreement {
            axis: AxisCode::Direction,
            valid_timeframes: 4,
            bullish_count: 1,
            bearish_count: 3,
            neutral_count: 0,
            majority: Some(MajorityClass::Bearish),
            majority_share: Some(0.75),
            agrees: true,
            insufficient_coverage: false,
        }],
        coverage,
        min_valid_timeframes: 2,
    };
    assert_eq!(agreement.of, 3);
    assert_eq!(agreement.agreeing_axes, 2);
    assert_eq!(agreement.axes[0].axis, AxisCode::Direction);
    assert_eq!(agreement.axes[0].majority, Some(MajorityClass::Bearish));
    assert_eq!(agreement.axes[0].valid_timeframes, 4);

    let churn = CrossTfChurn {
        max_changes_50: 6,
        sources: vec![ChurnSource {
            timeframe_secs: 3_600,
            axis: AxisCode::Direction,
        }],
        threshold: 6,
        window: 50,
        history_complete: true,
        churn_class: ChurnClass::Unstable,
        coverage,
    };
    assert_eq!(churn.max_changes_50, 6);
    assert_eq!(churn.churn_class, ChurnClass::Unstable);
    assert_eq!(churn.sources[0].timeframe_secs, 3_600);
}
