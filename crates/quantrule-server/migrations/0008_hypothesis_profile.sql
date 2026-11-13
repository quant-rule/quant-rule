-- A sealed hypothesis is scoped by profile. The fingerprint stays the content
-- identity; the row belongs to one profile and remains immutable.
ALTER TABLE research.hypotheses ADD COLUMN profile_id text NOT NULL DEFAULT 'default';
ALTER TABLE research.hypotheses DROP CONSTRAINT hypotheses_pkey;
ALTER TABLE research.hypotheses ADD PRIMARY KEY (profile_id, fingerprint);
ALTER TABLE research.hypotheses ALTER COLUMN profile_id DROP DEFAULT;
