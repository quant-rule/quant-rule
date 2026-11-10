# Blocked H1 Configs

- Date: 2026-11-10 (UTC)
- Status: Blocked by the event-composite engine

The H1 failed-break/reclaim rule needs a break event and a reclaim event with
opposite directions. The event-composite engine rejects opposite-direction entry
events at configuration time (`entry events disagree on direction`), and the
`contrarian` flag does not change that check. The compiled strategy registry also
exposes only each event's primary source, so `level.break`/`level.cross` at major
pivots (source 8) are not readable.

These configs are kept as evidence of the attempted rule. They are not runnable.
A sequence-aware entry condition, or a strategy-side source allow-list, is a
Requirements & Design change, not a strategy config.
