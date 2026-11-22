#!/usr/bin/env bash
# Assert the backtest worker's control-plane topology.
#
# The run and stage consumers must own separate RabbitMQ channels, each with its
# own prefetch slot, and the publisher channels must be in confirm mode. The
# audit found both consumers sharing one channel and QoS slot (AUD-41) and a
# publish that never waited for the broker (AUD-40).
#
# The queues must be the generated quorum generation: durable, with a delivery
# limit and a dead-letter exchange on every work queue, and a quorum dead-letter
# queue that has no dead-letter target of its own. The tier starts nothing: run
# it against the stack `make up` started. The output is parsed with jq, which
# the composed/release host provides; the image itself carries no interpreter.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

command -v jq >/dev/null || { echo "queue isolation: jq is required on the host" >&2; exit 1; }

consumers="$(docker compose exec -T rabbitmq rabbitmqctl list_consumers --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ service is not reachable; start the stack with 'make up'" >&2; exit 1; }
channels="$(docker compose exec -T rabbitmq rabbitmqctl list_channels pid user confirm consumer_count prefetch_count --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ channels are not readable" >&2; exit 1; }
queues="$(docker compose exec -T rabbitmq rabbitmqctl list_queues name type durable arguments --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ queues are not readable" >&2; exit 1; }
bindings="$(docker compose exec -T rabbitmq rabbitmqctl list_bindings source_name destination_name routing_key --formatter json 2>/dev/null)" \
  || { echo "queue isolation: the RabbitMQ bindings are not readable" >&2; exit 1; }
generation="$(docker compose exec -T backtest printenv QR_QUEUE_GENERATION 2>/dev/null || true)"
generation="${generation:-1}"
expected="$(docker compose exec -T backtest printenv QR_BACKTEST_CONCURRENCY 2>/dev/null || true)"
backtest="runs.backtest.g${generation}"
evidence="runs.evidence.g${generation}"
dead_letter="runs.dead-letter.g${generation}"
stage="stages.worker.g${generation}"

jq -en --argjson consumers "$consumers" --argjson channels "$channels" \
  --argjson queues "$queues" --argjson bindings "$bindings" \
  --argjson expected "${expected:-1}" --argjson generation "$generation" \
  --arg backtest "$backtest" --arg evidence "$evidence" \
  --arg dead_letter "$dead_letter" --arg stage "$stage" '
  def rows($queue): [$consumers[] | select(.queue_name == $queue)];
  def q($name): [$queues[] | select(.name == $name)][0];
  def arg($name; $key):
    [($queues[] | select(.name == $name) | .arguments[]
      | select(.[0] == $key) | .[2])][0];
  def work($name):
    q($name) != null
    and q($name).type == "quorum"
    and q($name).durable == true
    and arg($name; "x-queue-type") == "quorum"
    and arg($name; "x-delivery-limit") == 3
    and arg($name; "x-dead-letter-exchange") == "quantrule.runs.dlx";
  def bound($source; $destination; $key):
    any($bindings[];
      .source_name == $source
      and .destination_name == $destination
      and .routing_key == $key);

  if (work($backtest) and work($evidence) and work($stage)) | not then
    error("queue isolation: a work queue is not a durable quorum queue with the delivery limit and dead-letter exchange")
  else . end |
  if (q($dead_letter) != null
      and q($dead_letter).type == "quorum"
      and q($dead_letter).durable == true
      and arg($dead_letter; "x-queue-type") == "quorum"
      and arg($dead_letter; "x-delivery-limit") == null
      and arg($dead_letter; "x-dead-letter-exchange") == null) | not then
    error("queue isolation: the dead-letter queue is not a quorum queue with no dead-letter target")
  else . end |
  if (bound("quantrule.runs"; $dead_letter; "run.dead")
      and bound("quantrule.runs.dlx"; $dead_letter; "run.requested")
      and bound("quantrule.runs.dlx"; $dead_letter; "run.cancelled")
      and bound("quantrule.runs.dlx"; $dead_letter; "run.computed")
      and bound("quantrule.runs.dlx"; $dead_letter; "stage.requested")
      and bound("quantrule.runs.dlx"; $dead_letter; "stage.cancelled")) | not then
    error("queue isolation: the dead-letter queue is not bound for explicit and exhausted work")
  else . end |
  (rows($backtest)) as $runs |
  (rows($stage)) as $stages |
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
  "queue isolation: generation \($generation), \($runs | length) run consumer(s) and one stage consumer on \($used | length) channels; quorum work queues and DLQ verified; \($confirmed) publisher channel(s) confirm"
'
