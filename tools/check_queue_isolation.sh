#!/usr/bin/env bash
# Assert the backtest worker's control-plane topology.
#
# The run and stage consumers must own separate RabbitMQ channels, each with its
# own prefetch slot, and the publisher channels must be in confirm mode. The
# audit found both consumers sharing one channel and QoS slot (AUD-41) and a
# publish that never waited for the broker (AUD-40).
#
# The tier starts nothing: run it against the stack `make up` started.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

consumers="$(docker compose exec -T rabbitmq rabbitmqctl list_consumers --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ service is not reachable; start the stack with 'make up'" >&2; exit 1; }
channels="$(docker compose exec -T rabbitmq rabbitmqctl list_channels pid user confirm consumer_count prefetch_count --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ channels are not readable" >&2; exit 1; }
expected="$(docker compose exec -T backtest printenv QR_BACKTEST_CONCURRENCY 2>/dev/null || true)"

python3 - "$consumers" "$channels" "${expected:-1}" <<'PY'
import json
import sys

consumers = json.loads(sys.argv[1])
channels = json.loads(sys.argv[2])
expected = int(sys.argv[3].strip() or "1")


def rows(queue: str) -> list[dict]:
    return [row for row in consumers if row.get("queue_name") == queue]


runs = rows("runs.backtest")
stages = rows("stages.worker")
if len(runs) != expected:
    raise SystemExit(f"queue isolation: expected {expected} run consumer(s), found {len(runs)}")
if len(stages) != 1:
    raise SystemExit(f"queue isolation: expected one stage consumer, found {len(stages)}")
channels_used = {row["channel_pid"] for row in runs + stages}
if len(channels_used) != expected + 1:
    raise SystemExit(
        "queue isolation: the run and stage consumers share a channel: "
        f"{sorted(channels_used)}"
    )
if any(row["prefetch_count"] != 1 for row in runs + stages):
    raise SystemExit("queue isolation: a consumer does not own one prefetch slot")
confirmed = [row for row in channels if row.get("confirm") is True]
if not confirmed:
    raise SystemExit("queue isolation: no publisher channel is in confirm mode")

print(
    "queue isolation: "
    f"{len(runs)} run consumer(s) and one stage consumer on {len(channels_used)} channels; "
    f"{len(confirmed)} publisher channel(s) confirm"
)
PY
