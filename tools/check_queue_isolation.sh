#!/usr/bin/env bash
# Assert the backtest worker's control-plane topology.
#
# The run and stage consumers must own separate RabbitMQ channels, each with its
# own prefetch slot, and the publisher channels must be in confirm mode. The
# audit found both consumers sharing one channel and QoS slot (AUD-41) and a
# publish that never waited for the broker (AUD-40).
#
# The tier starts nothing: run it against the stack `make up` started. The
# output is parsed with jq, which the composed/release host provides; the image
# itself carries no interpreter.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

command -v jq >/dev/null || { echo "queue isolation: jq is required on the host" >&2; exit 1; }

consumers="$(docker compose exec -T rabbitmq rabbitmqctl list_consumers --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ service is not reachable; start the stack with 'make up'" >&2; exit 1; }
channels="$(docker compose exec -T rabbitmq rabbitmqctl list_channels pid user confirm consumer_count prefetch_count --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ channels are not readable" >&2; exit 1; }
expected="$(docker compose exec -T backtest printenv QR_BACKTEST_CONCURRENCY 2>/dev/null || true)"

jq -en --argjson consumers "$consumers" --argjson channels "$channels" \
  --argjson expected "${expected:-1}" '
  def rows($queue): [$consumers[] | select(.queue_name == $queue)];
  (rows("runs.backtest")) as $runs |
  (rows("stages.worker")) as $stages |
  if ($runs | length) != $expected then
    error("queue isolation: expected \($expected) run consumer(s), found \($runs | length)")
  else . end |
  if ($stages | length) != 1 then
    error("queue isolation: expected one stage consumer, found \($stages | length)")
  else . end |
  ([$runs[], $stages[]] | map(.channel_pid) | unique) as $used |
  if ($used | length) != ($expected + 1) then
    error("queue isolation: the run and stage consumers share a channel: \($used)")
  else . end |
  if ([$runs[], $stages[]] | any(.prefetch_count != 1)) then
    error("queue isolation: a consumer does not own one prefetch slot")
  else . end |
  ([$channels[] | select(.confirm == true)] | length) as $confirmed |
  if $confirmed == 0 then
    error("queue isolation: no publisher channel is in confirm mode")
  else . end |
  "queue isolation: \($runs | length) run consumer(s) and one stage consumer on \($used | length) channels; \($confirmed) publisher channel(s) confirm"
'
