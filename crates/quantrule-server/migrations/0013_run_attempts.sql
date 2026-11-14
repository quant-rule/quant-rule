-- The retry budget lives on the run row so a killed worker's redelivery is
-- observable and bounded even when the broker loses its own delivery count.
ALTER TABLE runs.runs ADD COLUMN attempts integer NOT NULL DEFAULT 0;
