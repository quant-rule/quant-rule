-- A profile is an organisational scope. The default exists before any client
-- asks, so a first run has somewhere to write.
CREATE TABLE IF NOT EXISTS catalog.profiles (
    profile_id text PRIMARY KEY,
    name       text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO catalog.profiles (profile_id, name)
VALUES ('default', 'Default')
ON CONFLICT (profile_id) DO NOTHING;
