//! The role registry for the model's readings.
//!
//! A reading is not owned by an axis. EMA 20, for example, participates in the
//! Direction alignment, acts as a dynamic structural level, is a series
//! crossover source for Signal events, and is an eligible reference for Level
//! events. The registry records every such use with its own definition, so a
//! consumer reads the definition of the role it is exercising rather than
//! treating the reading as belonging to one axis.
//!
//! A definition names the use and the contract section that specifies it. The
//! reading modules own the reading arithmetic; the axis modules own the axis
//! formulas. This table does not restate a formula as an independent
//! implementation, so it cannot drift into a second source of truth.
//!
//! The registry is metadata. It publishes the contract's source codes, axis
//! codes, and event-type codes, and it never enters the numeric market state.
//!
//! Contract §14 gives source codes 7 and 8 to confirmed minor and major pivots
//! and gives no separate code to a pivot candidate, while §8.4 makes the
//! candidate a reading in its own right. The registry therefore keys both
//! artifacts by the pivot family's code and carries [`PivotArtifact`] on each
//! pivot role. The event source code still means the confirmed pivot.

use super::PivotScale;

/// The contract's stable source code for a reading.
///
/// The values are fixed by the model contract §14 and are never reused. Codes 7
/// and 8 name the pivot family at each scale; the registry distinguishes the
/// candidate from the confirmed pivot with [`PivotArtifact`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadingCode {
    /// EMA 20.
    Ema20 = 1,
    /// EMA 50.
    Ema50 = 2,
    /// EMA 200.
    Ema200 = 3,
    /// RSI 14.
    Rsi14 = 4,
    /// ATR 14.
    Atr14 = 5,
    /// Relative volume 20.
    Rvol20 = 6,
    /// The minor swing-pivot family.
    PivotMinor = 7,
    /// The major swing-pivot family.
    PivotMajor = 8,
}

impl ReadingCode {
    /// Every reading code, in contract order.
    pub const ALL: [Self; 8] = [
        Self::Ema20,
        Self::Ema50,
        Self::Ema200,
        Self::Rsi14,
        Self::Atr14,
        Self::Rvol20,
        Self::PivotMinor,
        Self::PivotMajor,
    ];

    /// The contract's numeric source code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The pivot scale the code names, when it names a pivot family.
    #[must_use]
    pub const fn pivot_scale(self) -> Option<PivotScale> {
        match self {
            Self::PivotMinor => Some(PivotScale::Minor),
            Self::PivotMajor => Some(PivotScale::Major),
            _ => None,
        }
    }
}

/// The contract's numeric code for one of the four axes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AxisCode {
    /// Direction.
    Direction = 1,
    /// Momentum.
    Momentum = 2,
    /// Structure.
    Structure = 3,
    /// Progression.
    Progression = 4,
}

impl AxisCode {
    /// Every axis code, in contract order.
    pub const ALL: [Self; 4] = [
        Self::Direction,
        Self::Momentum,
        Self::Structure,
        Self::Progression,
    ];

    /// The contract's numeric axis code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// The contract's numeric code for an event type.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventTypeCode {
    /// A price/level interaction.
    Level = 1,
    /// A price/indicator relationship.
    Divergence = 2,
    /// A discrete change in a reading.
    Signal = 3,
}

impl EventTypeCode {
    /// Every event type, in contract order.
    pub const ALL: [Self; 3] = [Self::Level, Self::Divergence, Self::Signal];

    /// The contract's numeric event-type code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// The pivot artifact a role consumes.
///
/// A candidate is knowable on its occurrence bar; a confirmed pivot is
/// knowable only at its availability bar. The two are distinct readings, and a
/// role states which one it uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PivotArtifact {
    /// The candidate, available on its occurrence bar.
    Candidate,
    /// The confirmed pivot, available at its availability bar.
    Confirmed,
}

/// What a reading feeds.
///
/// The target states the consumer of one use. It is not an owner: a reading may
/// have several targets, and each carries its own definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoleTarget {
    /// The reading feeds the named axis through this definition.
    Axis(AxisCode),
    /// The reading is a source for the named event type.
    Event(EventTypeCode),
    /// The reading is published context and feeds no axis or event.
    Context,
}

impl RoleTarget {
    /// The axis this target names, when it names an axis.
    #[must_use]
    pub const fn axis(self) -> Option<AxisCode> {
        match self {
            Self::Axis(axis) => Some(axis),
            Self::Event(_) | Self::Context => None,
        }
    }

    /// The event type this target names, when it names an event type.
    #[must_use]
    pub const fn event_type(self) -> Option<EventTypeCode> {
        match self {
            Self::Event(event_type) => Some(event_type),
            Self::Axis(_) | Self::Context => None,
        }
    }
}

/// One recorded use of a reading.
///
/// The definition states what the reading contributes to that use and points to
/// the contract section that specifies it. It is deliberately local to the use:
/// the same reading may have a different definition for another axis or event
/// type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadingRole {
    /// The reading used.
    pub reading: ReadingCode,
    /// Which pivot artifact the use consumes, when the reading is a pivot family.
    pub artifact: Option<PivotArtifact>,
    /// The consumer of this use.
    pub target: RoleTarget,
    /// What the reading contributes to this use.
    pub definition: &'static str,
}

/// Every recorded use of every reading, in contract order.
///
/// The order is stable: readings first, then each reading's axes, event types,
/// and context. A consumer that needs a reading's uses selects them; it does not
/// take the first one as the reading's owner.
pub const READING_ROLES: &[ReadingRole] = &[
    ReadingRole {
        reading: ReadingCode::Ema20,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Direction),
        definition: "Direction alignment: the short leg of the relative EMA spread, scaled by ATR14 (contract 8.1)",
    },
    ReadingRole {
        reading: ReadingCode::Ema20,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Structure),
        definition: "Dynamic structural level: distance, crossing, and retest measurements (contract 8.3)",
    },
    ReadingRole {
        reading: ReadingCode::Ema20,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Signal),
        definition: "Series crossover with EMA50 or EMA200 (contract 7.4, subtype 2)",
    },
    ReadingRole {
        reading: ReadingCode::Ema20,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Level),
        definition: "Eligible level reference for test/retest, cross, and break events (contract 7.2)",
    },
    ReadingRole {
        reading: ReadingCode::Ema20,
        artifact: None,
        target: RoleTarget::Context,
        definition: "Progression context only; no progress arithmetic (contract 6)",
    },
    ReadingRole {
        reading: ReadingCode::Ema50,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Direction),
        definition: "Direction alignment: the long leg of the short spread and the short leg of the long spread, scaled by ATR14 (contract 8.1)",
    },
    ReadingRole {
        reading: ReadingCode::Ema50,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Structure),
        definition: "Dynamic structural level: distance, crossing, and retest measurements (contract 8.3)",
    },
    ReadingRole {
        reading: ReadingCode::Ema50,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Signal),
        definition: "Series crossover with EMA20 or EMA200 (contract 7.4, subtype 2)",
    },
    ReadingRole {
        reading: ReadingCode::Ema50,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Level),
        definition: "Eligible level reference for test/retest, cross, and break events (contract 7.2)",
    },
    ReadingRole {
        reading: ReadingCode::Ema50,
        artifact: None,
        target: RoleTarget::Context,
        definition: "Progression context only; no progress arithmetic (contract 6)",
    },
    ReadingRole {
        reading: ReadingCode::Ema200,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Direction),
        definition: "Direction alignment: the long leg of the long relative EMA spread, scaled by ATR14 (contract 8.1)",
    },
    ReadingRole {
        reading: ReadingCode::Ema200,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Structure),
        definition: "Dynamic structural level: distance, crossing, and retest measurements (contract 8.3)",
    },
    ReadingRole {
        reading: ReadingCode::Ema200,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Signal),
        definition: "Series crossover with EMA20 or EMA50 (contract 7.4, subtype 2)",
    },
    ReadingRole {
        reading: ReadingCode::Ema200,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Level),
        definition: "Eligible level reference for test/retest, cross, and break events (contract 7.2)",
    },
    ReadingRole {
        reading: ReadingCode::Ema200,
        artifact: None,
        target: RoleTarget::Context,
        definition: "Progression context only; no progress arithmetic (contract 6)",
    },
    ReadingRole {
        reading: ReadingCode::Rsi14,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Momentum),
        definition: "Signed force and its change; the source of momentum_force (contract 8.2)",
    },
    ReadingRole {
        reading: ReadingCode::Rsi14,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Divergence),
        definition: "RSI window around a confirmed pivot supplies R1 or R2 (contract 7.3)",
    },
    ReadingRole {
        reading: ReadingCode::Rsi14,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Signal),
        definition: "Threshold crossing of 30, 50, or 70 (contract 7.4, subtype 1)",
    },
    ReadingRole {
        reading: ReadingCode::Atr14,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Direction),
        definition: "Normalises both relative EMA spreads (contract 8.1)",
    },
    ReadingRole {
        reading: ReadingCode::Atr14,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Structure),
        definition: "Normalises EMA distance and sizes the retest zone (contract 8.3)",
    },
    ReadingRole {
        reading: ReadingCode::Atr14,
        artifact: None,
        target: RoleTarget::Axis(AxisCode::Progression),
        definition: "Normalises displacement and supplies the fallback target distance (contract 8.4)",
    },
    ReadingRole {
        reading: ReadingCode::Atr14,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Divergence),
        definition: "ATR14 at the newer pivot scales price_delta_atr (contract 7.3)",
    },
    ReadingRole {
        reading: ReadingCode::Atr14,
        artifact: None,
        target: RoleTarget::Event(EventTypeCode::Level),
        definition: "Scales level proximity and the break distance (contract 7.2)",
    },
    ReadingRole {
        reading: ReadingCode::Atr14,
        artifact: None,
        target: RoleTarget::Context,
        definition: "Volatility context ratio ATR14 / SMA20(ATR14) (contract 12)",
    },
    ReadingRole {
        reading: ReadingCode::Rvol20,
        artifact: None,
        target: RoleTarget::Context,
        definition: "Participation context; never votes on an axis and never defines progress (contracts 6 and 12)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMinor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Axis(AxisCode::Structure),
        definition: "Confirmed minor swing sequence and horizontal levels (contract 8.3)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMinor,
        artifact: Some(PivotArtifact::Candidate),
        target: RoleTarget::Axis(AxisCode::Progression),
        definition: "A minor candidate creates the forming leg in lifecycle 0 (contract 8.4)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMinor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Axis(AxisCode::Progression),
        definition: "A confirmed minor pivot confirms the leg origin, sets its direction, and supplies structural targets (contract 8.4)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMinor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Event(EventTypeCode::Level),
        definition: "The most recent confirmed minor swing high and low are level references (contract 7.2)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMinor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Event(EventTypeCode::Divergence),
        definition: "Consecutive confirmed minor pivots of the same kind supply P1 and P2 (contract 7.3)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMajor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Axis(AxisCode::Structure),
        definition: "Confirmed major swing sequence and horizontal levels (contract 8.3)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMajor,
        artifact: Some(PivotArtifact::Candidate),
        target: RoleTarget::Axis(AxisCode::Progression),
        definition: "A major candidate creates the forming leg in lifecycle 0 (contract 8.4)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMajor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Axis(AxisCode::Progression),
        definition: "A confirmed major pivot confirms the leg origin, sets its direction, and supplies structural targets (contract 8.4)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMajor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Event(EventTypeCode::Level),
        definition: "The most recent confirmed major swing high and low are level references (contract 7.2)",
    },
    ReadingRole {
        reading: ReadingCode::PivotMajor,
        artifact: Some(PivotArtifact::Confirmed),
        target: RoleTarget::Event(EventTypeCode::Divergence),
        definition: "Consecutive confirmed major pivots of the same kind supply P1 and P2 (contract 7.3)",
    },
];

/// The recorded uses of one reading, in registry order.
///
/// # Examples
///
/// ```
/// use quantrule_model::readings::roles::{ReadingCode, roles_for};
///
/// let ema20: Vec<_> = roles_for(ReadingCode::Ema20).collect();
/// assert!(ema20.len() > 1);
/// assert!(roles_for(ReadingCode::Rvol20).all(|role| role.target.axis().is_none()));
/// ```
pub fn roles_for(reading: ReadingCode) -> impl Iterator<Item = &'static ReadingRole> {
    READING_ROLES
        .iter()
        .filter(move |role| role.reading == reading)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        AxisCode, EventTypeCode, PivotArtifact, READING_ROLES, ReadingCode, RoleTarget, roles_for,
    };

    /// Every contracted use, independent of the production table. A missing or
    /// extra role fails this fixture.
    const EXPECTED: &[(ReadingCode, Option<PivotArtifact>, RoleTarget)] = &[
        (
            ReadingCode::Ema20,
            None,
            RoleTarget::Axis(AxisCode::Direction),
        ),
        (
            ReadingCode::Ema20,
            None,
            RoleTarget::Axis(AxisCode::Structure),
        ),
        (
            ReadingCode::Ema20,
            None,
            RoleTarget::Event(EventTypeCode::Signal),
        ),
        (
            ReadingCode::Ema20,
            None,
            RoleTarget::Event(EventTypeCode::Level),
        ),
        (ReadingCode::Ema20, None, RoleTarget::Context),
        (
            ReadingCode::Ema50,
            None,
            RoleTarget::Axis(AxisCode::Direction),
        ),
        (
            ReadingCode::Ema50,
            None,
            RoleTarget::Axis(AxisCode::Structure),
        ),
        (
            ReadingCode::Ema50,
            None,
            RoleTarget::Event(EventTypeCode::Signal),
        ),
        (
            ReadingCode::Ema50,
            None,
            RoleTarget::Event(EventTypeCode::Level),
        ),
        (ReadingCode::Ema50, None, RoleTarget::Context),
        (
            ReadingCode::Ema200,
            None,
            RoleTarget::Axis(AxisCode::Direction),
        ),
        (
            ReadingCode::Ema200,
            None,
            RoleTarget::Axis(AxisCode::Structure),
        ),
        (
            ReadingCode::Ema200,
            None,
            RoleTarget::Event(EventTypeCode::Signal),
        ),
        (
            ReadingCode::Ema200,
            None,
            RoleTarget::Event(EventTypeCode::Level),
        ),
        (ReadingCode::Ema200, None, RoleTarget::Context),
        (
            ReadingCode::Rsi14,
            None,
            RoleTarget::Axis(AxisCode::Momentum),
        ),
        (
            ReadingCode::Rsi14,
            None,
            RoleTarget::Event(EventTypeCode::Divergence),
        ),
        (
            ReadingCode::Rsi14,
            None,
            RoleTarget::Event(EventTypeCode::Signal),
        ),
        (
            ReadingCode::Atr14,
            None,
            RoleTarget::Axis(AxisCode::Direction),
        ),
        (
            ReadingCode::Atr14,
            None,
            RoleTarget::Axis(AxisCode::Structure),
        ),
        (
            ReadingCode::Atr14,
            None,
            RoleTarget::Axis(AxisCode::Progression),
        ),
        (
            ReadingCode::Atr14,
            None,
            RoleTarget::Event(EventTypeCode::Divergence),
        ),
        (
            ReadingCode::Atr14,
            None,
            RoleTarget::Event(EventTypeCode::Level),
        ),
        (ReadingCode::Atr14, None, RoleTarget::Context),
        (ReadingCode::Rvol20, None, RoleTarget::Context),
        (
            ReadingCode::PivotMinor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Axis(AxisCode::Structure),
        ),
        (
            ReadingCode::PivotMinor,
            Some(PivotArtifact::Candidate),
            RoleTarget::Axis(AxisCode::Progression),
        ),
        (
            ReadingCode::PivotMinor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Axis(AxisCode::Progression),
        ),
        (
            ReadingCode::PivotMinor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Event(EventTypeCode::Level),
        ),
        (
            ReadingCode::PivotMinor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Event(EventTypeCode::Divergence),
        ),
        (
            ReadingCode::PivotMajor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Axis(AxisCode::Structure),
        ),
        (
            ReadingCode::PivotMajor,
            Some(PivotArtifact::Candidate),
            RoleTarget::Axis(AxisCode::Progression),
        ),
        (
            ReadingCode::PivotMajor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Axis(AxisCode::Progression),
        ),
        (
            ReadingCode::PivotMajor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Event(EventTypeCode::Level),
        ),
        (
            ReadingCode::PivotMajor,
            Some(PivotArtifact::Confirmed),
            RoleTarget::Event(EventTypeCode::Divergence),
        ),
    ];

    #[test]
    fn reading_codes_match_the_contract() {
        assert_eq!(ReadingCode::Ema20.code(), 1);
        assert_eq!(ReadingCode::Ema50.code(), 2);
        assert_eq!(ReadingCode::Ema200.code(), 3);
        assert_eq!(ReadingCode::Rsi14.code(), 4);
        assert_eq!(ReadingCode::Atr14.code(), 5);
        assert_eq!(ReadingCode::Rvol20.code(), 6);
        assert_eq!(ReadingCode::PivotMinor.code(), 7);
        assert_eq!(ReadingCode::PivotMajor.code(), 8);
        assert_eq!(ReadingCode::ALL.len(), 8);
    }

    #[test]
    fn axis_and_event_codes_match_the_contract() {
        assert_eq!(AxisCode::Direction.code(), 1);
        assert_eq!(AxisCode::Momentum.code(), 2);
        assert_eq!(AxisCode::Structure.code(), 3);
        assert_eq!(AxisCode::Progression.code(), 4);
        assert_eq!(AxisCode::ALL.len(), 4);

        assert_eq!(EventTypeCode::Level.code(), 1);
        assert_eq!(EventTypeCode::Divergence.code(), 2);
        assert_eq!(EventTypeCode::Signal.code(), 3);
        assert_eq!(EventTypeCode::ALL.len(), 3);
    }

    #[test]
    fn pivot_reading_codes_carry_the_scale_code() {
        for reading in [ReadingCode::PivotMinor, ReadingCode::PivotMajor] {
            let scale = reading
                .pivot_scale()
                .expect("a pivot reading names a scale");
            assert_eq!(reading.code(), 6 + scale.code());
        }
        for reading in ReadingCode::ALL {
            if !matches!(reading, ReadingCode::PivotMinor | ReadingCode::PivotMajor) {
                assert_eq!(reading.pivot_scale(), None);
            }
        }
    }

    #[test]
    fn the_registry_matches_the_contracted_uses() {
        let actual: Vec<(ReadingCode, Option<PivotArtifact>, RoleTarget)> = READING_ROLES
            .iter()
            .map(|role| (role.reading, role.artifact, role.target))
            .collect();

        assert_eq!(actual, EXPECTED);
    }

    #[test]
    fn every_reading_has_at_least_one_recorded_use() {
        for reading in ReadingCode::ALL {
            assert!(
                roles_for(reading).next().is_some(),
                "{reading:?} has no recorded use"
            );
        }
    }

    #[test]
    fn every_pivot_family_records_both_artifacts() {
        for reading in [ReadingCode::PivotMinor, ReadingCode::PivotMajor] {
            let artifacts: BTreeSet<PivotArtifact> = roles_for(reading)
                .filter_map(|role| role.artifact)
                .collect();
            assert_eq!(
                artifacts,
                BTreeSet::from([PivotArtifact::Candidate, PivotArtifact::Confirmed])
            );
        }
        for reading in ReadingCode::ALL {
            if !matches!(reading, ReadingCode::PivotMinor | ReadingCode::PivotMajor) {
                assert!(
                    roles_for(reading).all(|role| role.artifact.is_none()),
                    "{reading:?} must not carry a pivot artifact"
                );
            }
        }
    }

    #[test]
    fn every_definition_is_non_empty() {
        for role in READING_ROLES {
            assert!(
                !role.definition.trim().is_empty(),
                "{:?} has an empty definition",
                role.reading
            );
        }
    }

    #[test]
    fn a_reading_is_not_owned_by_one_axis() {
        let axes = |reading: ReadingCode| -> BTreeSet<AxisCode> {
            roles_for(reading)
                .filter_map(|role| role.target.axis())
                .collect()
        };

        for reading in [
            ReadingCode::Ema20,
            ReadingCode::Ema50,
            ReadingCode::Ema200,
            ReadingCode::Atr14,
        ] {
            assert!(
                axes(reading).len() >= 2,
                "{reading:?} must serve more than one axis"
            );
        }
        assert!(axes(ReadingCode::PivotMinor).contains(&AxisCode::Progression));
        assert!(axes(ReadingCode::PivotMajor).contains(&AxisCode::Structure));
    }

    #[test]
    fn relative_volume_never_votes_on_an_axis() {
        for role in roles_for(ReadingCode::Rvol20) {
            assert_eq!(role.target, RoleTarget::Context);
        }
    }

    #[test]
    fn each_reading_artifact_and_target_is_recorded_once() {
        let mut seen = BTreeSet::new();
        for role in READING_ROLES {
            assert!(
                seen.insert((role.reading.code(), role.artifact, role.target)),
                "{:?} records {:?} for {:?} twice",
                role.reading,
                role.artifact,
                role.target
            );
        }
    }

    #[test]
    fn the_registry_covers_every_target_kind() {
        assert!(
            READING_ROLES
                .iter()
                .any(|role| role.target == RoleTarget::Context)
        );
        for axis in AxisCode::ALL {
            assert!(
                READING_ROLES
                    .iter()
                    .any(|role| role.target == RoleTarget::Axis(axis)),
                "{axis:?} has no reading"
            );
        }
        for event_type in EventTypeCode::ALL {
            assert!(
                READING_ROLES
                    .iter()
                    .any(|role| role.target == RoleTarget::Event(event_type)),
                "{event_type:?} has no source reading"
            );
        }
    }

    #[test]
    fn roles_for_preserves_registry_order_and_repeats_identically() {
        let first: Vec<_> = roles_for(ReadingCode::PivotMajor).collect();
        let second: Vec<_> = roles_for(ReadingCode::PivotMajor).collect();

        assert_eq!(first, second);
        for (role, expected) in first.iter().zip(
            READING_ROLES
                .iter()
                .filter(|role| role.reading == ReadingCode::PivotMajor),
        ) {
            assert_eq!(*role, expected);
        }
    }
}
