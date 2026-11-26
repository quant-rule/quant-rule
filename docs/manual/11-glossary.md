# 11. Glossary

- Status: Current
- Audience: any reader who needs one term, not a second contract

One term per concept. A term is not a synonym for its neighbour; the distinction
is the point.

| Term | One-sentence meaning |
|---|---|
| Data version | The content identity of a normalized OHLCV dataset, written as `data_version`. |
| Config id | The content identity of one run configuration bundle, written as `config_id`. |
| Config hash | The SHA-256 of the config bundle that a run records as provenance. |
| Stage | One diagnostic engine execution over one input, never a coupled evidence run. |
| Stage artifact | The content-addressed file bundle a stage publishes, with no canonical digest. |
| Run | One coupled execution of data, model, strategy, execution, portfolio, and evidence. |
| Run id | The content identity of a run, derived from data, config, model version, and image digest, plus the frozen model artifact id for a promoted run. |
| Canonical digest | The SHA-256 over the sorted `(path, sha256)` list for `results.json`, `*.csv`, every `metrics.json`, and `*.ndjson`. |
| Canonical artifact set | The result files a canonical digest covers, excluding provenance, manifests, logs, and timestamps. |
| Evidence | A succeeded coupled run whose artifacts and canonical digest are intact. |
| Diagnostic | A stage artifact or read-only inspection used for diagnosis, never evidence. |
| Governed evidence | A pre-registered improvement run whose coupled result and human decision are recorded. |
| Baseline | An accepted improvement result for one tier, stored append-only with its run and scope. |
| Invalidation | An append-only tombstone that records a baseline or scope removed from use. |
| Improvement id | The server-assigned identity of one v3 improvement-run pre-registration. |
| Partition | A named date range a tier uses for development, validation, or holdout. |
| Holdout | The one-shot portfolio partition used after a candidate is frozen. |
| Promoted run | A coupled run that reuses a succeeded model artifact and recomputes the rest. |
| Model artifact | The content-addressed model bundle a succeeded model stage publishes. |
| Decisions artifact | The content-addressed decisions bundle a succeeded strategy stage publishes. |
| Profile | An organisational scope for records; it is not a security boundary. |
| Token | The bearer secret a gated gateway requires; it is a network gate, not an account. |
| Retention | The configured days before diagnostic stage artifacts expire; evidence is permanent. |

## Terms that are not synonyms

| Pair | Distinction |
|---|---|
| Stage / run | A stage is one diagnostic engine; a run is the coupled evidence path. |
| Stage artifact / canonical artifact set | A stage artifact is diagnostic; the canonical set is the run result the digest covers. |
| Run id / canonical digest | The run id identifies the execution; the digest identifies the result artifact bytes. |
| Evidence / governed evidence | Evidence is a coupled run; governed evidence also has a pre-registration and a human decision. |
| Baseline / invalidation | A baseline is accepted for use; an invalidation is a tombstone that removes a baseline or scope. |
| Profile / token | A profile organises records; a token gates network access. |
| Retention / deletion | Retention expires diagnostic stage artifacts; it never deletes a run or research record. |

## How to read a record

| Field | Read it as |
|---|---|
| `data_version` | Which candles |
| `config_hash` | Which configuration bundle |
| `model_version` | Which model identity the config names |
| `image_digest` | Which build executed the run |
| `run_id` | Which execution, derived from the four values above |
| `digest` | Which result bytes, independent of provenance and time |

A missing value is `null` or `none`, never zero. A valid zero is a value.

The contract and the stored record are authoritative; this glossary only names
the concept.

