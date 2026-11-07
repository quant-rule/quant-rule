//! The registered event identities and their strategy roles.
//!
//! The table is generated at build time from `docs/registry/indicators.json`.
//! The strategy engines use it to reject an unregistered event identity and to
//! keep a diagnostic event out of a decision. The registry remains the one
//! definition; this module is only the compiled lookup.

/// The role of a registered event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventRole {
    /// A signal event may enter a strategy rule.
    Signal,
    /// A context event may filter a strategy rule.
    Context,
    /// A diagnostic event is never readable by a strategy.
    Diagnostic,
}

/// One registered event identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventIdentity {
    /// The event type code: 1 level, 2 divergence, 3 signal.
    pub event_type: u8,
    /// The registered subtype code.
    pub subtype: u8,
    /// The registered source code.
    pub source: u16,
    /// The registered role.
    pub role: EventRole,
}

include!(concat!(env!("OUT_DIR"), "/strategy_registry.rs"));

/// The registered compression-range event identity.
///
/// The compression-bracket engine reads the range from this event and no other.
pub const COMPRESSION_RANGE_EVENT: (u8, u8, u16) = (3, 8, 5);

/// Looks up one registered event identity.
#[must_use]
pub fn lookup(event_type: u8, subtype: u8, source: u16) -> Option<EventIdentity> {
    let key = (event_type, subtype, source);
    let index = EVENTS
        .binary_search_by_key(&key, |row| (row.0, row.1, row.2))
        .ok()?;
    let row = EVENTS[index];
    Some(EventIdentity {
        event_type: row.0,
        subtype: row.1,
        source: row.2,
        role: row.3,
    })
}

/// Whether an event identity is registered.
#[must_use]
pub fn is_registered(event_type: u8, subtype: u8, source: u16) -> bool {
    lookup(event_type, subtype, source).is_some()
}

/// Whether a role may enter a strategy decision.
#[must_use]
pub const fn is_strategy_role(role: EventRole) -> bool {
    matches!(role, EventRole::Signal | EventRole::Context)
}
