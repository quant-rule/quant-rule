# 12. Troubleshooting

- Status: Current
- Audience: operator reading a stable error code and choosing the next action

The shell prints `error: <code>: <message>` and exits nonzero. The browser shows
the same code in a notice and clears the affected rows rather than presenting
stale data. The token code is `auth.unauthorized`; there is no `token.invalid`.

## Error families

| Code | Cause | Client behavior | Next action |
|---|---|---|---|
| `auth.unauthorized` | Missing or wrong bearer token on a gated gateway | Shell prints the code; browser shows `token required` | Enter the token on Connection, or pass `--token` / `QUANTRULE_TOKEN` |
| `profile.not_found` | Missing or unknown profile header | Shell refuses the call; browser clears profile rows | Create the profile or select an existing one |
| `profile.invalid` | Profile name or id failed validation | The create form reports the code | Use lowercase letters, digits, and hyphens |
| `dataset.not_found` | Unknown `data_version` in this profile | The affected read or run is refused | Re-read `dataset list`; do not construct an id |
| `dataset.invalid` | Blank name, bad UTF-8, or a rejected candle | Upload is refused; tolerant mode reports skipped rows | Fix the file, or choose tolerant deliberately |
| `config.invalid` | Schema or cross-config rule failed | The config shows `invalid`; submit is disabled | Read the parser message; fix the config |
| `hypothesis.not_found` | Unknown sealed hypothesis fingerprint | The read is refused | Re-read the hypothesis source; do not guess a fingerprint |
| `hypothesis.invalid` | Hypothesis failed validation | The write is refused | Fix the hypothesis JSON |
| `run.not_found` | Unknown run id in this profile | The run detail is unavailable | Re-read `run list` |
| `run.invalid` | Malformed id, dataset, or config reference | The call is refused | Re-read both ids from their lists |
| `run.model_artifact_invalid` | Missing, non-succeeded, or mismatched frozen model artifact | Submit is refused | Re-run the model stage or choose a matching artifact |
| `run.conflict` | Cancel after a terminal state | The cancel is refused | Read `run status`; do not retry |
| `stage.not_found` | Unknown stage id or upstream artifact | The stage read is refused | Re-read `stage list`; use a succeeded stage in this profile |
| `stage.invalid` | Bad stage input, config, or provenance | Submit or read is refused | Read the message; fix the reference |
| `stage.conflict` | Terminal stage, or no artifact yet | Cancel/delete/events may be refused | Read `stage show`; wait or submit a new stage |
| `research.not_found` | Unknown improvement id or tier baseline | The record read is refused | Re-read `research list` or `baseline list` |
| `research.invalid` | Bad manifest, partition, config, dataset, criteria, or note | Create or decision is refused | Fix the pre-registration; the note is required |
| `research.conflict` | Duplicate submit/decision, or accepted without a succeeded digest | The action is refused | Read the record; wait for the run or choose a non-accepted decision |
| `upload.too_large` | Body exceeds the configured limit | Upload is refused | Split the source or raise the limit with the human |
| `queue.unavailable` | Broker unreachable | Submission or cancellation is refused | Read `health`; stop until the broker returns |
| `service.unavailable` | Downstream service unreachable or refused the proxied call | The call is refused; rows are not presented as fresh | Read `health` and `make logs`; stop |
| `simulation.not_found` | Unknown simulation id | The run is unavailable | Re-read `simulation list` |
| `simulation.mark_invalid` | Bar outside the window, or a schema violation | The mark is refused; the note is kept | Select a bar in the window; fix the mark fields |
| `simulation.check_incomplete` | Fewer than 30 events produced a check row; the sample is underpowered | The check is refused | Use a bundle or timeframe with at least 30 events; do not mark the same small sample harder |
| `simulation.locked` | Marking after a decision | The chart is locked | Do not mark; start a new simulation if needed |
| `internal` | Unclassified server defect | The call fails with `internal` | Record the request, check logs, and report the defect |

## Connection symptoms

| Symptom | Meaning | Next action |
|---|---|---|
| `unreachable` | The gateway does not answer | Check the URL, `make ps`, and the private network |
| `version mismatch` | The client and gateway API versions differ | Use a client built for the gateway version |
| `token required` | The gateway enforces a token and none was accepted | Enter the token on Connection |
| `degraded` | A dependency is down but the gateway answers | Read the service table; stop before submitting |
| Rows disappear after a profile switch | The scope was cleared, not a data loss | Re-read under the new profile |
| A run stays `queued` | No worker is consuming the queue | Check the backtest worker and queue; stop |
| A message is in the dead-letter queue | The handler rejected it past the delivery limit | Read the queue name, `x-death`, and the run row; do not republish blindly |

## What a code does not mean

A stable error is not a research result, a failed hypothesis, or an edge verdict.
Fix the request, the configuration, or the dependency, then repeat the same
procedure. Do not change a threshold or a rule to turn an error into a pass.

## Related

- [Command reference](09-command-reference.md)
- [GUI reference](10-gui-reference.md)
- [Operations](08-operations.md)
- [HTTP API](../architecture/http-api.md)
