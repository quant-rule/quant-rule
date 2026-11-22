-- The persistent research pack registry.
--
-- A pack is content-addressed by the SHA-256 of its canonical bytes. The row is
-- profile-scoped metadata and identity: the object store is the single
-- authority for the bytes, and this table records the object key, hash and size.
-- Lifecycle state is metadata outside the hash and every transition is
-- append-only, so a freeze cannot be rewritten and a retired pack is never
-- deleted.
CREATE TABLE IF NOT EXISTS catalog.packs (
    profile_id   text NOT NULL,
    pack_id      text NOT NULL,
    kind         text NOT NULL CHECK (kind IN ('strategy', 'portfolio', 'execution')),
    name         text NOT NULL,
    version      text NOT NULL,
    owner        text NOT NULL,
    state        text NOT NULL CHECK (state IN ('draft', 'validated', 'frozen', 'retired')),
    object_key   text NOT NULL,
    sha256       text NOT NULL,
    bytes        bigint NOT NULL CHECK (bytes >= 0),
    created_at   timestamptz NOT NULL DEFAULT now(),
    validated_at timestamptz,
    frozen_at    timestamptz,
    retired_at   timestamptz,
    PRIMARY KEY (profile_id, pack_id),
    UNIQUE (profile_id, name, version),
    CHECK (length(pack_id) = 71),
    CHECK (sha256 = pack_id),
    CHECK (state <> 'validated' OR validated_at IS NOT NULL),
    CHECK (state <> 'frozen' OR frozen_at IS NOT NULL),
    CHECK (state <> 'retired' OR retired_at IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS catalog.pack_validation_records (
    record_id  bigserial PRIMARY KEY,
    profile_id text NOT NULL,
    pack_id    text NOT NULL,
    validator  text NOT NULL,
    valid      boolean NOT NULL,
    message    text,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (profile_id, pack_id)
        REFERENCES catalog.packs (profile_id, pack_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS pack_validation_records_pack_idx
    ON catalog.pack_validation_records (profile_id, pack_id, record_id);

CREATE TABLE IF NOT EXISTS catalog.pack_lifecycle_events (
    event_id   bigserial PRIMARY KEY,
    profile_id text NOT NULL,
    pack_id    text NOT NULL,
    from_state text CHECK (from_state IN ('draft', 'validated', 'frozen', 'retired')),
    to_state   text NOT NULL CHECK (to_state IN ('draft', 'validated', 'frozen', 'retired')),
    action     text NOT NULL CHECK (action IN ('create', 'validate', 'freeze', 'retire')),
    reason     text,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (profile_id, pack_id)
        REFERENCES catalog.packs (profile_id, pack_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS pack_lifecycle_events_pack_idx
    ON catalog.pack_lifecycle_events (profile_id, pack_id, event_id);
