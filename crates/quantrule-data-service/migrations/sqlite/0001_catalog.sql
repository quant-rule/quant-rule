-- The catalog's SQLite baseline for the embedded profile.
--
-- This is the final logical shape, not a translation of the Postgres history:
-- a desktop database starts at v1.1 and has no earlier rows to migrate. SQLite
-- has no schemas, so `catalog.datasets` is `catalog__datasets`. A JSON document
-- is TEXT checked by `json_valid`, an instant is RFC 3339 UTC TEXT, and a
-- boolean is 0 or 1.
--
-- A profile scopes the row; the object the dataset points at and the config
-- bundle a config points at are shared by content, exactly as in the Postgres
-- shape. The checks repeat the Postgres constraints so a malformed row is
-- refused at the boundary instead of surfacing as a decode error later.

CREATE TABLE IF NOT EXISTS catalog__datasets (
    profile_id   TEXT NOT NULL,
    data_version TEXT NOT NULL,
    name         TEXT NOT NULL,
    candles      INTEGER NOT NULL CHECK (candles >= 0),
    first_bar    INTEGER NOT NULL,
    last_bar     INTEGER NOT NULL,
    bytes        INTEGER NOT NULL CHECK (bytes >= 0),
    object_key   TEXT NOT NULL,
    sha256       TEXT NOT NULL,
    quality      TEXT CHECK (quality IS NULL OR json_valid(quality)),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (profile_id, data_version),
    CHECK (first_bar <= last_bar)
) STRICT;

CREATE TABLE IF NOT EXISTS catalog__config_bundles (
    config_id TEXT PRIMARY KEY,
    files     TEXT NOT NULL CHECK (json_valid(files))
) STRICT;

CREATE TABLE IF NOT EXISTS catalog__configs (
    profile_id  TEXT NOT NULL,
    config_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    origin      TEXT NOT NULL CHECK (origin IN ('repository', 'upload')),
    valid       INTEGER NOT NULL CHECK (valid IN (0, 1)),
    message     TEXT,
    config_hash TEXT,
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (profile_id, config_id),
    CHECK ((valid = 1 AND config_hash IS NOT NULL) OR (valid = 0))
) STRICT;
