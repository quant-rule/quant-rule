# SIM-10 — Opening-range sweep (blocked)

The M1.3 2018–2022 reports have no session block, so
`signal.opening_range_sweep` (144) and `signal.opening_range_sweep_control`
(145) are absent in every year. The event count is zero.

The run does not substitute `level.break` or synthesize the event. A
session-enabled report is a separate Requirements & Design / Model Change. The
blocked record is kept in the canonical store for completeness.
