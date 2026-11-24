# Repository Tools

- Status: Current; the product image and the repository carry no Python
- Gate: [`tools/gate.sh`](gate.sh)
- Checkers: `qr-governance check --all` (the Rust registry is the single authority)

These tools are repository governance. They are not part of the product runtime,
and the product image contains no interpreter. The checkers run in the workspace
gate and in CI.

| Tool | Purpose | Path |
|---|---|---|
| Checker set | Runs every registered repository checker. | `qr-governance check --all` |
| Change-scope planner | Classifies a change range as docs, governance, web, or full. | `qr-governance gate plan` |
| Scoped gate | Runs the full gate or one classified scope. | `tools/gate.sh --changed` |
| Worktree cleanup guard | Classifies, archives, and removes a merged task worktree; required before `git worktree remove`. | `qr-governance worktree` |
| Verification manifest builder | Regenerates `verification.json` and both verification matrices from the registry and claims. | `qr-governance verification build` |
| Registry generator | Rewrites the generated registry Markdown from `indicators.json`. | `qr-governance registry --write` |
| Label table | Checks or rewrites the generated client label table. | `qr-governance labels generate [--write]` |
| Python guard | Refuses any `.py` file or interpreter reference in the repository tree. | `qr-governance guard python` |
| Structure report | Reports module size, public surface, and layout metrics. | `qr-governance structure --report` |
| Secret files | Creates the local mode-700 secret directory and mode-600 files. | `tools/secrets.sh` |
| Internal PKI | Creates the offline-root/intermediate/180-day-leaf bundle, SANs and service layouts. | `tools/pki.sh` |
| Swarm control plane | Validates/renders the v3 HA descriptor and wraps bootstrap, status, upgrade, snapshot, rollback, and the read-only remote/overlay check. | `tools/swarm.sh` |
| Stateful tier | Renders, creates the overlay, copies configs/PKI/secrets, starts etcd -> Patroni -> HAProxy, waits for quorum/leader/sync, and optionally sets the data-directory owner. `--plan` is Docker-free and no value is printed. | `tools/stateful.sh` |
| Observability tier | Renders, copies, starts, waits, and verifies Prometheus, Alertmanager, Loki, promtail, blackbox, node_exporter, and Grafana; writes the manager-quorum textfile. `--plan` is Docker/SSH/jq/secret-free. | `tools/observability.sh` |
| Queue rebuild | Imports stored topology into an empty broker, recreates the app user/vhost, restarts evidence so the run and stage outboxes re-arm, and prints counts and the DLQ path; never purges, deletes, or publishes work. `--plan` is Docker/SSH/jq/secret-free and `--confirm` is required live. | `tools/queue-rebuild.sh` |
| Object archive | Copies new/changed keys one way with `mc mirror --overwrite --summary` from the read-only `qr_archive_reader` source to the operator target, writes a per-key SHA-256 manifest and value-free evidence with a redacted target, and never deletes the source, watches it, or claims replication/version history. `--plan` is Docker/jq/mc/secret-free; `--restore` mirrors into a fresh scratch bucket. | `tools/object-archive.sh` |
| Backup/DR drill | Restores a fresh scratch PGDATA and bucket, or the disposable lab in place with `--confirm` and a pre-restore backup, rebuilds the queue, reruns the same run/config, compares the canonical digest, and writes `quantrule.dr/v1` RPO/RTO evidence; `--plan` is Docker/SSH/jq/secret-free and every skip leaves its row `Unverified`. | `deploy/v3/dr.sh` |
| Release evidence | Prints the offline release plan; generates the SPDX SBOM from an exact digest; gates Grype plus `cargo audit` with an expiring allowlist; signs a digest into a Sigstore bundle plus `cosign save` signed image and verifies it offline; writes an OCI bundle with the real bundles and saved image; pins a descriptor image without touching the 16 infrastructure pins. | `tools/release.sh` |
| Reviewer verification | Submits two equivalent runs through the gateway and prints one canonical digest; `--cacert`/`QUANTRULE_CA_CERT` trusts a private CA. | `tools/verify.sh` |
| HA lab tier | Opt-in three-VM Swarm deploy, worker/manager/VIP loss, Patroni-primary pause, HAProxy pause, RTO, marker/digest/rejoin evidence, and the observability phase (probe/job baseline, promtool rule unit, manager/gateway alerts); `--plan` is Docker-free and a missing lab prints `UNVERIFIED`. | `deploy/v3/test.sh` |
| Lab harness | Preflights release mode, provisions stateful -> swarm -> observability -> the remote/overlay/runtime check, runs the lab-only failure harness, and runs the client TLS/token phase; `--plan` is Docker/SSH/jq/secret-free, the app/MinIO fixtures stop it at the named field with `Unverified`, and evidence is value-free JSON. | `deploy/v3/lab.sh` |
| Composed tier | Checks a running Compose stack's gateway, then submits two equivalent runs and compares their canonical digest. | `tools/composed.sh` |
| Project-sync test | Checks that `Done` is terminal in the board status decision. | `tools/test_project_sync.sh` |

## Governance CLI

`crates/quantrule-governance` builds `qr-governance`, a Rust developer tool for
repository governance and historical evidence. It is not a product client, is
not in the product image, and never reads a service.

| Command | Purpose |
|---|---|
| `qr-governance check --all` | Run every repository checker |
| `qr-governance check --checker <name>` | Run one checker |
| `qr-governance check --list` | List the checker names |
| `qr-governance gate plan --changed <range>` | Classify a change range for the scoped gate |
| `qr-governance history list --kind config\|docs\|records` | List historical files with bytes and date |
| `qr-governance history archive --kind ... --out ... (--before ... \| --all)` | Copy a selected set and write a verified manifest |
| `qr-governance history erase --manifest ... --confirm ...` | Erase only a manifest that still matches the source bytes |
| `qr-governance verification build` | Regenerate the evidence manifest and both matrices |
| `qr-governance labels generate [--write]` | Check or rewrite the generated client label table |
| `qr-governance guard python` | Refuse Python in the repository tree |
| `qr-governance release note --tag <tag>` | Resolve a current tag to its release note |
| `qr-governance improvement ...` | Validate, index, and run improvement records |
| `qr-governance research ...` | Validate, run, and report research programs |

The historical Python checker, data, research, and improvement stacks are
retired. Git history is the recovery path. The boundary is recorded in
[`tools/improvement/README.md`](improvement/README.md),
[`tools/experiment/README.md`](experiment/README.md),
[`tools/research/README.md`](research/README.md), and
[`tools/data/README.md`](data/README.md).
