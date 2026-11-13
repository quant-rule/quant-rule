-- The sealed record is stored whole, so a reader reconstructs exactly what was
-- written. The split columns remain for indexing and inspection.
ALTER TABLE research.hypotheses ADD COLUMN body jsonb NOT NULL;
