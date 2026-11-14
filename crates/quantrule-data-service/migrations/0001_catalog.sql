-- Inputs: datasets and configurations. Both are immutable once written.
CREATE SCHEMA IF NOT EXISTS catalog;

CREATE TABLE IF NOT EXISTS catalog.datasets (
    data_version text PRIMARY KEY,
    name         text NOT NULL,
    candles      bigint NOT NULL CHECK (candles >= 0),
    first_bar    bigint NOT NULL,
    last_bar     bigint NOT NULL,
    bytes        bigint NOT NULL CHECK (bytes >= 0),
    object_key   text NOT NULL,
    sha256       text NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CHECK (first_bar <= last_bar)
);

CREATE TABLE IF NOT EXISTS catalog.configs (
    config_id   text PRIMARY KEY,
    name        text NOT NULL,
    origin      text NOT NULL CHECK (origin IN ('repository', 'upload')),
    valid       boolean NOT NULL,
    message     text,
    config_hash text,
    body        text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    CHECK ((valid AND config_hash IS NOT NULL) OR (NOT valid))
);
