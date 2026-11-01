//! `strategy.json`: the human-owned unified ladder rule.
//!
//! The strategy selects the active ladder, declares per-axis context weights,
//! states the execution and context entry conditions, chooses the exit reasons,
//! and carries the risk intent. It contains no fill mechanics, indicator
//! formula, statistic, cash, or P&L.
//!
//! The v3 schema replaces the retired `axis-agreement` v2 schema. A v2 file is
//! rejected with a migration message.

use quantrule_backtest::context::{LadderConfig, LadderWeights};
use quantrule_backtest::intent::RiskPolicy;
use quantrule_backtest::strategy::{
    ContextEntryConfig, EntryConfig, ExecutionEntryConfig, ExitConfig, LevelRetestConfig,
    StrategyConfig, ThreeTimeframeConfig,
};
use quantrule_data::Timeframe;
use quantrule_model::readings::PivotScale;
use serde_json::{Map, Value};

use super::data::timeframe;
use super::error::ConfigError;
use super::read::{self, ROOT};

/// The v4 level-retest entry fields that do not belong to the common entry.
#[derive(Debug, Clone, Copy)]
struct LevelEntry {
    max_distance_fraction: f64,
    scale: Option<PivotScale>,
}

/// The parsed risk shape.
#[derive(Debug, Clone, Copy)]
enum Risk {
    Atr(RiskPolicy),
    Level {
        stop_buffer: f64,
        min_reward_to_risk: f64,
    },
}

/// The resolved strategy blueprint.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StrategyRunConfig {
    /// The engine configuration and risk intent.
    pub(crate) engine: StrategyConfig,
    /// The human explanation of why the rule should have an edge.
    pub(crate) thesis: String,
}

/// The accepted strategy schema versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Schema {
    V3,
    V4,
    V5,
    V6,
    V7,
}

/// Parses `strategy.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<StrategyRunConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    // Check the schema before the field set so a real v2 file reaches the
    // migration message instead of failing on its retired fields.
    let schema = require_schema(file, object)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema", "rule", "thesis", "ladder", "weights", "entry", "exit", "risk", "no_trade",
        ],
    )?;

    match schema {
        Schema::V3 => require_code(
            file,
            "$.rule",
            read::field(file, ROOT, object, "rule")?,
            "ladder-context",
        )?,
        Schema::V4 => require_code(
            file,
            "$.rule",
            read::field(file, ROOT, object, "rule")?,
            "level-retest",
        )?,
        Schema::V5 => require_code(
            file,
            "$.rule",
            read::field(file, ROOT, object, "rule")?,
            "three-timeframe",
        )?,
        Schema::V6 => require_code(
            file,
            "$.rule",
            read::field(file, ROOT, object, "rule")?,
            "breakout",
        )?,
        Schema::V7 => require_code(
            file,
            "$.rule",
            read::field(file, ROOT, object, "rule")?,
            "level-fade",
        )?,
    }
    let thesis =
        read::non_empty_string(file, "$.thesis", read::field(file, ROOT, object, "thesis")?)?
            .to_owned();

    let (ladder, context_codes) = parse_ladder(file, read::field(file, ROOT, object, "ladder")?)?;
    let weights = parse_weights(file, object, &context_codes)?;
    let (entry, level, min_agreement) =
        parse_entry(file, read::field(file, ROOT, object, "entry")?, schema)?;
    let exit = parse_exit(file, read::field(file, ROOT, object, "exit")?, schema)?;
    let risk = parse_risk(file, read::field(file, ROOT, object, "risk")?, schema)?;
    let require_contiguous_execution =
        parse_no_trade(file, read::field(file, ROOT, object, "no_trade")?)?;

    let engine = match (schema, level, min_agreement, risk) {
        (Schema::V3, None, None, Risk::Atr(risk)) => StrategyConfig::new(
            ladder,
            weights,
            entry,
            exit,
            risk,
            require_contiguous_execution,
        ),
        (
            Schema::V4,
            Some(level),
            None,
            Risk::Level {
                stop_buffer,
                min_reward_to_risk,
            },
        ) => {
            let level = LevelRetestConfig::new(
                level.max_distance_fraction,
                stop_buffer,
                min_reward_to_risk,
                level.scale,
            )
            .map_err(|error| {
                ConfigError::range(
                    file,
                    "$.entry.level",
                    "a valid level-retest configuration",
                    &error.to_string(),
                )
            })?;
            StrategyConfig::new_level_retest(
                ladder,
                weights,
                entry,
                exit,
                level,
                require_contiguous_execution,
            )
        }
        (
            Schema::V5,
            Some(level),
            Some(min_agreement),
            Risk::Level {
                stop_buffer,
                min_reward_to_risk,
            },
        ) => {
            let level = LevelRetestConfig::new(
                level.max_distance_fraction,
                stop_buffer,
                min_reward_to_risk,
                level.scale,
            )
            .map_err(|error| {
                ConfigError::range(
                    file,
                    "$.entry.level",
                    "a valid level-retest configuration",
                    &error.to_string(),
                )
            })?;
            let three = ThreeTimeframeConfig::new(level, min_agreement).map_err(|error| {
                ConfigError::range(
                    file,
                    "$.entry.agreement",
                    "a valid agreement configuration",
                    &error.to_string(),
                )
            })?;
            StrategyConfig::new_three_timeframe(
                ladder,
                weights,
                entry,
                exit,
                three,
                require_contiguous_execution,
            )
        }
        (
            Schema::V6,
            Some(level),
            None,
            Risk::Level {
                stop_buffer,
                min_reward_to_risk,
            },
        ) => {
            let level = LevelRetestConfig::new(
                level.max_distance_fraction,
                stop_buffer,
                min_reward_to_risk,
                level.scale,
            )
            .map_err(|error| {
                ConfigError::range(
                    file,
                    "$.entry.level",
                    "a valid breakout configuration",
                    &error.to_string(),
                )
            })?;
            StrategyConfig::new_breakout(
                ladder,
                weights,
                entry,
                exit,
                level,
                require_contiguous_execution,
            )
        }
        (
            Schema::V7,
            Some(level),
            None,
            Risk::Level {
                stop_buffer,
                min_reward_to_risk,
            },
        ) => {
            let level = LevelRetestConfig::new(
                level.max_distance_fraction,
                stop_buffer,
                min_reward_to_risk,
                level.scale,
            )
            .map_err(|error| {
                ConfigError::range(
                    file,
                    "$.entry.level",
                    "a valid level-fade configuration",
                    &error.to_string(),
                )
            })?;
            StrategyConfig::new_level_fade(
                ladder,
                weights,
                entry,
                exit,
                level,
                require_contiguous_execution,
            )
        }
        _ => {
            return Err(ConfigError::inconsistent(
                file,
                "$.schema",
                "a supported strategy shape",
                "the parsed shape",
            ));
        }
    }
    .map_err(|error| {
        ConfigError::range(
            file,
            "$",
            "a valid strategy configuration",
            &error.to_string(),
        )
    })?;

    Ok(StrategyRunConfig { engine, thesis })
}

fn require_schema(file: &str, object: &Map<String, Value>) -> Result<Schema, ConfigError> {
    let path = read::child(ROOT, "schema");
    let found = read::string(file, &path, read::field(file, ROOT, object, "schema")?)?;
    if found == "quantrule.strategy/v2" {
        return Err(ConfigError::range(
            file,
            &path,
            "quantrule.strategy/v2 is retired; use quantrule.strategy/v3 with rule \"ladder-context\" or v4 with rule \"level-retest\"",
            found,
        ));
    }
    match found {
        "quantrule.strategy/v3" => Ok(Schema::V3),
        "quantrule.strategy/v4" => Ok(Schema::V4),
        "quantrule.strategy/v5" => Ok(Schema::V5),
        "quantrule.strategy/v6" => Ok(Schema::V6),
        "quantrule.strategy/v7" => Ok(Schema::V7),
        _ => Err(ConfigError::schema(
            file,
            &path,
            "quantrule.strategy/v3, v4, v5, v6, or v7",
            found,
        )),
    }
}

fn parse_ladder(
    file: &str,
    value: &Value,
) -> Result<(LadderConfig, Vec<&'static str>), ConfigError> {
    let path = "$.ladder";
    let array = read::array(file, path, value)?;
    if array.len() < 2 {
        return Err(ConfigError::range(
            file,
            path,
            "at least two timeframes",
            &array.len().to_string(),
        ));
    }
    let mut timeframes = Vec::with_capacity(array.len());
    for (index, item) in array.iter().enumerate() {
        timeframes.push(timeframe(file, &read::index(path, index), item)?);
    }
    let execution = timeframes[0];
    let context: Vec<Timeframe> = timeframes[1..].to_vec();
    let execution_secs = u64::from(execution.seconds());
    let context_secs: Vec<u64> = context
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect();
    let ladder = LadderConfig::new(execution_secs, context_secs).map_err(|error| {
        ConfigError::range(file, path, "an ascending active ladder", &error.to_string())
    })?;
    let codes = context.iter().map(|timeframe| timeframe.code()).collect();
    Ok((ladder, codes))
}

fn parse_weights(
    file: &str,
    object: &Map<String, Value>,
    context_codes: &[&'static str],
) -> Result<LadderWeights, ConfigError> {
    let path = "$.weights";
    let Some(value) = object.get("weights") else {
        return Ok(LadderWeights::equal(context_codes.len()));
    };
    let weights = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        weights,
        &["direction", "momentum", "structure", "progression"],
    )?;

    let direction = weights_block(file, weights, "direction", context_codes)?;
    let momentum = weights_block(file, weights, "momentum", context_codes)?;
    let structure = weights_block(file, weights, "structure", context_codes)?;
    let progression = weights_block(file, weights, "progression", context_codes)?;

    LadderWeights::new(direction, momentum, structure, progression).map_err(|error| {
        ConfigError::range(
            file,
            path,
            "positive finite axis weights",
            &error.to_string(),
        )
    })
}

fn weights_block(
    file: &str,
    weights: &Map<String, Value>,
    axis: &str,
    context_codes: &[&'static str],
) -> Result<Vec<f64>, ConfigError> {
    let path = read::child("$.weights", axis);
    let Some(value) = weights.get(axis) else {
        return Ok(vec![1.0; context_codes.len()]);
    };
    let block = read::object(file, &path, value)?;
    read::check_fields(file, &path, block, context_codes)?;
    let mut values = Vec::with_capacity(context_codes.len());
    for code in context_codes {
        let weight_path = read::child(&path, code);
        let weight = read::number(file, &weight_path, read::field(file, &path, block, code)?)?;
        if !weight.is_finite() || weight <= 0.0 {
            return Err(ConfigError::range(
                file,
                &weight_path,
                "> 0",
                &weight.to_string(),
            ));
        }
        values.push(weight);
    }
    Ok(values)
}

fn parse_entry(
    file: &str,
    value: &Value,
    schema: Schema,
) -> Result<(EntryConfig, Option<LevelEntry>, Option<u64>), ConfigError> {
    let path = "$.entry";
    let entry = read::object(file, path, value)?;
    let fields: &[&str] = match schema {
        Schema::V3 => &["execution", "context"],
        Schema::V4 | Schema::V6 | Schema::V7 => &["execution", "context", "level"],
        Schema::V5 => &["execution", "context", "level", "agreement"],
    };
    read::check_fields(file, path, entry, fields)?;
    let execution = parse_execution_entry(file, read::field(file, path, entry, "execution")?)?;
    let context = parse_context_entry(file, read::field(file, path, entry, "context")?)?;
    let level = match schema {
        Schema::V3 => None,
        Schema::V4 | Schema::V5 | Schema::V6 | Schema::V7 => Some(parse_level_entry(
            file,
            read::field(file, path, entry, "level")?,
        )?),
    };
    let min_agreement = match schema {
        Schema::V5 => Some(parse_agreement_entry(
            file,
            read::field(file, path, entry, "agreement")?,
        )?),
        Schema::V3 | Schema::V4 | Schema::V6 | Schema::V7 => None,
    };
    Ok((EntryConfig::new(execution, context), level, min_agreement))
}

fn parse_agreement_entry(file: &str, value: &Value) -> Result<u64, ConfigError> {
    let path = "$.entry.agreement";
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["min_bars"])?;
    let min_bars = read::u64(
        file,
        "$.entry.agreement.min_bars",
        read::field(file, path, object, "min_bars")?,
    )?;
    if min_bars == 0 {
        return Err(ConfigError::range(
            file,
            "$.entry.agreement.min_bars",
            "at least one bar",
            "0",
        ));
    }
    Ok(min_bars)
}

fn parse_level_entry(file: &str, value: &Value) -> Result<LevelEntry, ConfigError> {
    let path = "$.entry.level";
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["max_distance_fraction", "scale"])?;
    let max_distance_fraction = read::number(
        file,
        "$.entry.level.max_distance_fraction",
        read::field(file, path, object, "max_distance_fraction")?,
    )?;
    let scale = read::string(
        file,
        "$.entry.level.scale",
        read::field(file, path, object, "scale")?,
    )?;
    let scale = match scale {
        "both" => None,
        "minor" => Some(PivotScale::Minor),
        "major" => Some(PivotScale::Major),
        other => {
            return Err(ConfigError::unsupported(
                file,
                "$.entry.level.scale",
                "both, minor, or major",
                other,
            ));
        }
    };
    Ok(LevelEntry {
        max_distance_fraction,
        scale,
    })
}

fn parse_execution_entry(file: &str, value: &Value) -> Result<ExecutionEntryConfig, ConfigError> {
    let path = "$.entry.execution";
    let object = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        object,
        &[
            "direction_min",
            "momentum_min",
            "structure_min",
            "structure_shift",
        ],
    )?;
    let direction_min = read::number(
        file,
        "$.entry.execution.direction_min",
        read::field(file, path, object, "direction_min")?,
    )?;
    let momentum_min = read::number(
        file,
        "$.entry.execution.momentum_min",
        read::field(file, path, object, "momentum_min")?,
    )?;
    let structure_min = read::number(
        file,
        "$.entry.execution.structure_min",
        read::field(file, path, object, "structure_min")?,
    )?;

    let shift_path = "$.entry.execution.structure_shift";
    let shift_value = read::field(file, path, object, "structure_shift")?;
    let shift = read::object(file, shift_path, shift_value)?;
    read::check_fields(file, shift_path, shift, &["from_max", "to_min"])?;
    let from_max = integer(
        file,
        "$.entry.execution.structure_shift.from_max",
        read::field(file, shift_path, shift, "from_max")?,
    )?;
    let to_min = integer(
        file,
        "$.entry.execution.structure_shift.to_min",
        read::field(file, shift_path, shift, "to_min")?,
    )?;
    let from_max = i8::try_from(from_max).map_err(|_| {
        ConfigError::range(
            file,
            "$.entry.execution.structure_shift.from_max",
            "an integer in [-2, 2]",
            &from_max.to_string(),
        )
    })?;
    let to_min = i8::try_from(to_min).map_err(|_| {
        ConfigError::range(
            file,
            "$.entry.execution.structure_shift.to_min",
            "an integer in [-2, 2]",
            &to_min.to_string(),
        )
    })?;

    ExecutionEntryConfig::new(direction_min, momentum_min, structure_min, from_max, to_min).map_err(
        |error| {
            ConfigError::range(
                file,
                path,
                "a valid execution entry configuration",
                &error.to_string(),
            )
        },
    )
}

fn parse_context_entry(file: &str, value: &Value) -> Result<ContextEntryConfig, ConfigError> {
    let path = "$.entry.context";
    let object = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        object,
        &["direction", "momentum", "structure", "progression"],
    )?;

    let direction_path = "$.entry.context.direction";
    let direction = read::object(
        file,
        direction_path,
        read::field(file, path, object, "direction")?,
    )?;
    read::check_fields(file, direction_path, direction, &["kind", "share_min"])?;
    require_code(
        file,
        "$.entry.context.direction.kind",
        read::field(file, direction_path, direction, "kind")?,
        "bullish",
    )?;
    let share_min = read::number(
        file,
        "$.entry.context.direction.share_min",
        read::field(file, direction_path, direction, "share_min")?,
    )?;

    let momentum_path = "$.entry.context.momentum";
    let momentum = read::object(
        file,
        momentum_path,
        read::field(file, path, object, "momentum")?,
    )?;
    read::check_fields(
        file,
        momentum_path,
        momentum,
        &["kind", "bearish_share_max"],
    )?;
    require_code(
        file,
        "$.entry.context.momentum.kind",
        read::field(file, momentum_path, momentum, "kind")?,
        "not_bearish",
    )?;
    let bearish_share_max = read::number(
        file,
        "$.entry.context.momentum.bearish_share_max",
        read::field(file, momentum_path, momentum, "bearish_share_max")?,
    )?;

    let structure_path = "$.entry.context.structure";
    let structure = read::object(
        file,
        structure_path,
        read::field(file, path, object, "structure")?,
    )?;
    read::check_fields(file, structure_path, structure, &["conflict_count_max"])?;
    let conflict_count_max = u32(
        file,
        "$.entry.context.structure.conflict_count_max",
        read::field(file, structure_path, structure, "conflict_count_max")?,
    )?;

    let progression_path = "$.entry.context.progression";
    let progression = read::object(
        file,
        progression_path,
        read::field(file, path, object, "progression")?,
    )?;
    read::check_fields(
        file,
        progression_path,
        progression,
        &["direction", "late_or_terminal_count_max"],
    )?;
    require_code(
        file,
        "$.entry.context.progression.direction",
        read::field(file, progression_path, progression, "direction")?,
        "bullish",
    )?;
    let late_or_terminal_count_max = u32(
        file,
        "$.entry.context.progression.late_or_terminal_count_max",
        read::field(
            file,
            progression_path,
            progression,
            "late_or_terminal_count_max",
        )?,
    )?;

    ContextEntryConfig::new(
        share_min,
        bearish_share_max,
        conflict_count_max,
        late_or_terminal_count_max,
    )
    .map_err(|error| {
        ConfigError::range(
            file,
            path,
            "a valid context entry configuration",
            &error.to_string(),
        )
    })
}

fn parse_exit(file: &str, value: &Value, schema: Schema) -> Result<ExitConfig, ConfigError> {
    let path = "$.exit";
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["on"])?;
    let array = read::array(file, "$.exit.on", read::field(file, path, object, "on")?)?;
    if array.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.exit.on",
            "at least one exit reason",
            "empty",
        ));
    }

    let mut context_lost = false;
    let mut structure_lost = false;
    let mut level_lost = false;
    for (index, item) in array.iter().enumerate() {
        let item_path = read::index("$.exit.on", index);
        let reason = read::string(file, &item_path, item)?;
        match reason {
            "context_lost" => {
                if context_lost {
                    return Err(ConfigError::inconsistent(
                        file,
                        &item_path,
                        "a unique exit reason",
                        reason,
                    ));
                }
                context_lost = true;
            }
            "structure_lost" => {
                if structure_lost {
                    return Err(ConfigError::inconsistent(
                        file,
                        &item_path,
                        "a unique exit reason",
                        reason,
                    ));
                }
                structure_lost = true;
            }
            "level_lost"
                if schema == Schema::V4
                    || schema == Schema::V5
                    || schema == Schema::V6
                    || schema == Schema::V7 =>
            {
                if level_lost {
                    return Err(ConfigError::inconsistent(
                        file,
                        &item_path,
                        "a unique exit reason",
                        reason,
                    ));
                }
                level_lost = true;
            }
            other => {
                let expected = match schema {
                    Schema::V3 => "context_lost or structure_lost",
                    Schema::V4 | Schema::V5 | Schema::V6 | Schema::V7 => {
                        "context_lost, structure_lost, or level_lost"
                    }
                };
                return Err(ConfigError::unsupported(file, &item_path, expected, other));
            }
        }
    }

    let config = match schema {
        Schema::V3 => ExitConfig::new(context_lost, structure_lost),
        Schema::V4 | Schema::V5 | Schema::V6 | Schema::V7 => {
            ExitConfig::new_level(context_lost, structure_lost, level_lost)
        }
    };
    config.map_err(|error| {
        ConfigError::range(file, path, "a valid exit configuration", &error.to_string())
    })
}

fn parse_risk(file: &str, value: &Value, schema: Schema) -> Result<Risk, ConfigError> {
    let path = "$.risk";
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["stop", "target"])?;

    let stop_path = "$.risk.stop";
    let stop = read::object(file, stop_path, read::field(file, path, object, "stop")?)?;
    let target_path = "$.risk.target";
    let target = read::object(
        file,
        target_path,
        read::field(file, path, object, "target")?,
    )?;

    match schema {
        Schema::V3 => {
            read::check_fields(file, stop_path, stop, &["kind", "multiple"])?;
            require_code(
                file,
                "$.risk.stop.kind",
                read::field(file, stop_path, stop, "kind")?,
                "atr",
            )?;
            let stop_multiple = read::number(
                file,
                "$.risk.stop.multiple",
                read::field(file, stop_path, stop, "multiple")?,
            )?;
            read::check_fields(file, target_path, target, &["kind", "multiple"])?;
            require_code(
                file,
                "$.risk.target.kind",
                read::field(file, target_path, target, "kind")?,
                "reward_to_risk",
            )?;
            let reward_to_risk = read::number(
                file,
                "$.risk.target.multiple",
                read::field(file, target_path, target, "multiple")?,
            )?;
            RiskPolicy::new(stop_multiple, reward_to_risk)
                .map(Risk::Atr)
                .map_err(|error| {
                    ConfigError::range(file, path, "a valid risk policy", &error.to_string())
                })
        }
        Schema::V4 | Schema::V5 | Schema::V6 | Schema::V7 => {
            read::check_fields(file, stop_path, stop, &["kind", "buffer_fraction"])?;
            require_code(
                file,
                "$.risk.stop.kind",
                read::field(file, stop_path, stop, "kind")?,
                "level",
            )?;
            let stop_buffer = read::number(
                file,
                "$.risk.stop.buffer_fraction",
                read::field(file, stop_path, stop, "buffer_fraction")?,
            )?;
            read::check_fields(file, target_path, target, &["kind", "min_reward_to_risk"])?;
            require_code(
                file,
                "$.risk.target.kind",
                read::field(file, target_path, target, "kind")?,
                "next_level",
            )?;
            let min_reward_to_risk = read::number(
                file,
                "$.risk.target.min_reward_to_risk",
                read::field(file, target_path, target, "min_reward_to_risk")?,
            )?;
            Ok(Risk::Level {
                stop_buffer,
                min_reward_to_risk,
            })
        }
    }
}

fn parse_no_trade(file: &str, value: &Value) -> Result<bool, ConfigError> {
    let path = "$.no_trade";
    let object = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        object,
        &["missing_context", "require_contiguous_execution"],
    )?;
    require_code(
        file,
        "$.no_trade.missing_context",
        read::field(file, path, object, "missing_context")?,
        "reject",
    )?;
    let contiguous = read::boolean(
        file,
        "$.no_trade.require_contiguous_execution",
        read::field(file, path, object, "require_contiguous_execution")?,
    )?;
    if !contiguous {
        return Err(ConfigError::range(
            file,
            "$.no_trade.require_contiguous_execution",
            "true",
            "false",
        ));
    }
    Ok(contiguous)
}

fn require_code(file: &str, path: &str, value: &Value, expected: &str) -> Result<(), ConfigError> {
    let found = read::string(file, path, value)?;
    if found != expected {
        return Err(ConfigError::unsupported(file, path, expected, found));
    }
    Ok(())
}

fn integer(file: &str, path: &str, value: &Value) -> Result<i64, ConfigError> {
    value
        .as_i64()
        .ok_or_else(|| ConfigError::wrong_type(file, path, "an integer", &read::observed(value)))
}

fn u32(file: &str, path: &str, value: &Value) -> Result<u32, ConfigError> {
    let number = read::u64(file, path, value)?;
    u32::try_from(number).map_err(|_| {
        ConfigError::range(
            file,
            path,
            "an integer that fits in 32 bits",
            &number.to_string(),
        )
    })
}
