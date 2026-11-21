# Data tooling

- Status: Retired; the committed source CSVs are the audit record
- Replacement: data-service ingestion (`quantrule fetch`) and `qr-governance check --checker partitions`
- Historical record: the data-change records under `docs/config/` and Git history

The Python Binance importer and coverage auditor were retired with the
zero-Python migration. They built and audited source CSVs offline and never
entered the signal path. The committed source files, manifests, and data-change
records remain the audit trail; Git history is the recovery path.
