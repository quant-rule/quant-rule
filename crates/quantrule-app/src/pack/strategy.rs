//! The strategy pack payload and its content-addressed identity.
//!
//! A strategy pack is the human-owned alpha definition. The envelope and the
//! payload are immutable; the only mutable field is lifecycle state, which is
//! outside the canonical bytes. Validation reuses the run strategy parser's
//! engine construction, so the registered event identities, diagnostic roles,
//! state ranges, and risk bounds have one definition.
//!
//! Signal mode excludes the fold-only giveback fields. The closed schema
//! rejects every field it does not define, which is the rule-purity control:
//! a statistical, fitted, or machine-learning parameter has no field to travel
//! in.

use quantrule_api::Digest;
use quantrule_data::Timeframe;
use serde_json::{Map, Value};

use crate::config::read;
use crate::config::{ConfigError, StrategyConfig, StrategyEngine, parse_risk, parse_timeframe};

use super::canonical::canonical_json;
use super::envelope::{PackEnvelope, PackKind, PackMetadata};
use super::error::PackError;
use super::lifecycle::{PackState, PackStateError};

/// The reusable or fold-only execution mode of a strategy pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrategyMode {
    /// A reusable signal artifact is computed from the pack; giveback is
    /// excluded because candles are not a signal-service input.
    Signal,
    /// The strategy runs inside the backtest fold with the actual position.
    Fold,
}

impl StrategyMode {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Signal => "signal",
            Self::Fold => "fold",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "signal" => Some(Self::Signal),
            "fold" => Some(Self::Fold),
            _ => None,
        }
    }
}

/// The validated payload of a strategy pack.
#[derive(Debug, Clone)]
pub struct StrategyPackPayload {
    mode: StrategyMode,
    universe: Vec<String>,
    strategy: StrategyConfig,
}

impl StrategyPackPayload {
    fn parse(file: &str, path: &str, value: &Value) -> Result<Self, PackError> {
        let object = read::object(file, path, value)?;
        let mode_code = read::string(
            file,
            &read::child(path, "mode"),
            read::field(file, path, object, "mode")?,
        )?;
        let mode = StrategyMode::from_code(mode_code).ok_or_else(|| {
            ConfigError::range(
                file,
                &read::child(path, "mode"),
                "signal or fold",
                mode_code,
            )
        })?;
        let engine_name = read::string(
            file,
            &read::child(path, "engine"),
            read::field(file, path, object, "engine")?,
        )?;
        let allowed: &[&str] = match engine_name {
            "event_composite" => &[
                "mode",
                "engine",
                "execution_timeframe",
                "context_timeframes",
                "universe",
                "event_window_bars",
                "time_stop_bars",
                "giveback_atr",
                "giveback_arm_atr",
                "require_contiguous",
                "risk",
                "entry",
                "exit",
            ],
            "compression_bracket" => &[
                "mode",
                "engine",
                "execution_timeframe",
                "context_timeframes",
                "universe",
                "time_stop_bars",
                "require_contiguous",
                "risk",
                "event",
                "entry_valid_bars",
            ],
            other => {
                return Err(ConfigError::range(
                    file,
                    &read::child(path, "engine"),
                    "event_composite or compression_bracket",
                    other,
                )
                .into());
            }
        };
        read::check_fields(file, path, object, allowed)?;

        let execution_timeframe = parse_timeframe(
            file,
            &read::child(path, "execution_timeframe"),
            read::field(file, path, object, "execution_timeframe")?,
        )?;
        let context_timeframes = parse_context_timeframes(
            file,
            &read::child(path, "context_timeframes"),
            object,
            execution_timeframe,
        )?;
        let universe = parse_universe(file, &read::child(path, "universe"), object)?;
        validate_mode_fields(file, path, object, mode, engine_name)?;

        let mut engine_object = object.clone();
        match mode {
            StrategyMode::Signal => {
                let _ = engine_object.remove("giveback_atr");
                let _ = engine_object.remove("giveback_arm_atr");
            }
            StrategyMode::Fold => normalize_fold_giveback(object, &mut engine_object),
        }
        let engine = StrategyEngine::parse(file, path, &engine_object, execution_timeframe)?;
        let risk = parse_risk(
            file,
            &read::child(path, "risk"),
            read::field(file, path, object, "risk")?,
        )?;
        let require_contiguous = object
            .get("require_contiguous")
            .map(|value| read::boolean(file, &read::child(path, "require_contiguous"), value))
            .transpose()?
            .unwrap_or(true);
        let strategy = StrategyConfig::from_parts(
            None,
            execution_timeframe,
            context_timeframes,
            engine,
            risk,
            require_contiguous,
        );
        Ok(Self {
            mode,
            universe,
            strategy,
        })
    }

    /// The execution mode.
    #[must_use]
    pub const fn mode(&self) -> StrategyMode {
        self.mode
    }

    /// The optional universe constraint, empty when unconstrained.
    #[must_use]
    pub fn universe(&self) -> &[String] {
        &self.universe
    }

    /// The validated pipeline strategy.
    ///
    /// Its `id` is unset because the pack envelope name is the human label.
    #[must_use]
    pub fn strategy(&self) -> &StrategyConfig {
        &self.strategy
    }
}

/// A strategy pack with its canonical identity and lifecycle state.
#[derive(Debug, Clone)]
pub struct StrategyPack {
    metadata: PackMetadata,
    payload: StrategyPackPayload,
    canonical: String,
    id: Digest,
    state: PackState,
}

impl StrategyPack {
    /// Parses and validates a strategy pack document.
    ///
    /// The content hash is taken over the canonical bytes of the whole
    /// envelope, including the name, version, owner, and payload. The pack
    /// starts in [`PackState::Draft`].
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the envelope is invalid, the kind is not
    /// `strategy`, or the payload disagrees with the strategy schema.
    pub fn parse(value: &Value, file: &str) -> Result<Self, PackError> {
        let envelope = PackEnvelope::parse(value, file)?;
        if envelope.metadata().kind() != PackKind::Strategy {
            return Err(PackError::WrongKind {
                expected: PackKind::Strategy,
                found: envelope.metadata().kind(),
            });
        }
        let payload = StrategyPackPayload::parse(file, "$.payload", envelope.payload())?;
        let canonical = canonical_json(value);
        let id = Digest::sha256(canonical.as_bytes());
        Ok(Self {
            metadata: envelope.metadata().clone(),
            payload,
            canonical,
            id,
            state: PackState::Draft,
        })
    }

    /// Parses a JSON text document.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the text is not JSON or the document is not a
    /// valid strategy pack.
    pub fn parse_json(text: &str, file: &str) -> Result<Self, PackError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|error| ConfigError::invalid_json(file, &error.to_string()))?;
        Self::parse(&value, file)
    }

    /// Reloads a pack from stored canonical bytes.
    ///
    /// The bytes are hashed as stored; they are never re-serialised. Bytes that
    /// parse but differ from the canonical form are refused, so a stored pack
    /// cannot acquire a second identity through whitespace or key order.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the bytes are not UTF-8, are not JSON, are not
    /// a valid strategy pack, or are not canonical.
    pub fn from_stored_bytes(bytes: &[u8], file: &str) -> Result<Self, PackError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ConfigError::invalid_json(file, "the stored pack is not UTF-8"))?;
        let mut pack = Self::parse_json(text, file)?;
        if pack.canonical.as_bytes() != bytes {
            return Err(PackError::NonCanonical {
                file: file.to_owned(),
            });
        }
        // The stored bytes are the identity, not a fresh serialisation of them.
        pack.id = Digest::sha256(bytes);
        Ok(pack)
    }

    /// The immutable envelope metadata.
    #[must_use]
    pub fn metadata(&self) -> &PackMetadata {
        &self.metadata
    }

    /// The validated payload.
    #[must_use]
    pub fn payload(&self) -> &StrategyPackPayload {
        &self.payload
    }

    /// The content hash of the canonical bytes.
    #[must_use]
    pub fn id(&self) -> &Digest {
        &self.id
    }

    /// The canonical bytes the identity covers.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.canonical.as_bytes()
    }

    /// The canonical JSON text, including its trailing newline.
    #[must_use]
    pub fn canonical_text(&self) -> &str {
        &self.canonical
    }

    /// The lifecycle state.
    #[must_use]
    pub const fn state(&self) -> PackState {
        self.state
    }

    /// Marks the pack validated after the schema check.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is a draft.
    pub fn validate(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Validated)
    }

    /// Rejects a validated pack and returns it to draft for editing.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is validated.
    pub fn reject(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Draft)
    }

    /// Freezes a validated pack.
    ///
    /// The caller owns the human freeze decision and its Quant Validator
    /// record; this method enforces the lifecycle precondition only.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is validated.
    pub fn freeze(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Frozen)
    }

    /// Retires a frozen pack from new runs.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is frozen.
    pub fn retire(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Retired)
    }

    fn transition(&mut self, to: PackState) -> Result<(), PackStateError> {
        let from = self.state;
        if from.can_transition_to(to) {
            self.state = to;
            Ok(())
        } else {
            Err(PackStateError::new(from, to))
        }
    }
}

fn parse_context_timeframes(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
    execution: Timeframe,
) -> Result<Vec<Timeframe>, ConfigError> {
    let Some(value) = object.get("context_timeframes") else {
        return Ok(Vec::new());
    };
    let values = read::array(file, path, value)?;
    let mut timeframes = Vec::with_capacity(values.len());
    let mut previous: Option<u32> = None;
    for (index, value) in values.iter().enumerate() {
        let item_path = read::index(path, index);
        let timeframe = parse_timeframe(file, &item_path, value)?;
        if timeframe == execution {
            return Err(ConfigError::range(
                file,
                &item_path,
                "a context timeframe different from the execution timeframe",
                timeframe.code(),
            ));
        }
        if previous.is_some_and(|seconds| timeframe.seconds() <= seconds) {
            return Err(ConfigError::range(
                file,
                &item_path,
                "a strictly ascending context timeframe",
                timeframe.code(),
            ));
        }
        previous = Some(timeframe.seconds());
        timeframes.push(timeframe);
    }
    Ok(timeframes)
}

fn parse_universe(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
) -> Result<Vec<String>, ConfigError> {
    let Some(value) = object.get("universe") else {
        return Ok(Vec::new());
    };
    let values = read::array(file, path, value)?;
    let mut universe = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let item_path = read::index(path, index);
        let instrument = read::non_empty_string(file, &item_path, value)?;
        if universe
            .iter()
            .any(|existing: &String| existing == instrument)
        {
            return Err(ConfigError::range(
                file,
                &item_path,
                "a unique instrument",
                instrument,
            ));
        }
        universe.push(instrument.to_owned());
    }
    Ok(universe)
}

fn validate_mode_fields(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
    mode: StrategyMode,
    engine_name: &str,
) -> Result<(), ConfigError> {
    match mode {
        StrategyMode::Signal => {
            if let Some(value) = object.get("giveback_atr")
                && !value.is_null()
            {
                return Err(ConfigError::unsupported(
                    file,
                    &read::child(path, "giveback_atr"),
                    "null in signal mode",
                    &read::observed(value),
                ));
            }
            if let Some(value) = object.get("giveback_arm_atr") {
                let arm = read::number(file, &read::child(path, "giveback_arm_atr"), value)?;
                if arm != 0.0 {
                    return Err(ConfigError::unsupported(
                        file,
                        &read::child(path, "giveback_arm_atr"),
                        "0.0 in signal mode",
                        &read::observed(value),
                    ));
                }
            }
        }
        StrategyMode::Fold if engine_name == "event_composite" => {
            let threshold = object.get("giveback_atr");
            if let Some(value) = object.get("giveback_arm_atr") {
                let arm = read::number(file, &read::child(path, "giveback_arm_atr"), value)?;
                if arm < 0.0 {
                    return Err(ConfigError::range(
                        file,
                        &read::child(path, "giveback_arm_atr"),
                        "a non-negative arm",
                        &read::observed(value),
                    ));
                }
                let disabled = threshold.is_none_or(Value::is_null);
                if arm > 0.0 && disabled {
                    return Err(ConfigError::inconsistent(
                        file,
                        &read::child(path, "giveback_arm_atr"),
                        "a non-null giveback_atr threshold",
                        &read::observed(value),
                    ));
                }
            }
        }
        StrategyMode::Fold => {}
    }
    Ok(())
}

fn normalize_fold_giveback(object: &Map<String, Value>, engine_object: &mut Map<String, Value>) {
    if object.get("giveback_atr").is_none_or(Value::is_null) {
        // A null or absent threshold disables the exit. Removing the arm keeps
        // the engine parser from accepting a value it would silently ignore.
        let _ = engine_object.remove("giveback_atr");
        let _ = engine_object.remove("giveback_arm_atr");
    }
}

#[cfg(test)]
mod tests;
