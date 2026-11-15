-- The gateway's SQLite baseline for the embedded profile.
--
-- This is the final logical shape, not a translation of the Postgres history:
-- a desktop database starts at v1.1 and has no earlier rows to migrate. SQLite
-- has no schemas, so `gateway.profiles` is `gateway__profiles`. The instant is
-- RFC 3339 UTC text, the SQLite spelling of `timestamptz`.
--
-- The default row is what gives a first run somewhere to write, exactly as the
-- Postgres migration does. A second apply leaves the row alone.
CREATE TABLE IF NOT EXISTS gateway__profiles (
    profile_id TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT OR IGNORE INTO gateway__profiles (profile_id, name)
VALUES ('default', 'Default');
