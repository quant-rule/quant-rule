#!/usr/bin/env bash
# The v3 chaos cases: worker, evidence leader, non-VIP gateway, etcd, partial
# publish, and digest parity. Sourced by `deploy/v3/chaos.sh`.

chaos_gateway_kill() {
  local holder target container start host
  holder=$(ingress_holder)
  [ -n "$holder" ] || chaos_fail "no ingress host holds the VIP"
  target=""
  for host in $ingress_hosts; do
    if [ "$host" != "$holder" ]; then target=$host; break; fi
  done
  [ -n "$target" ] || chaos_fail "no non-VIP ingress host"
  container=$(chaos_task_on_host gateway "$target")
  start=$SECONDS
  chaos_kill_on_host "$target" "$container"
  targeted_check "after gateway SIGKILL"
  wait_converged
  chaos_write HA-01 "Simulated HA" gateway-sigkill "$((SECONDS - start))" \
    "sha256:$chaos_digest" "the non-VIP gateway task was SIGKILLed; ingress stayed healthy and the task restarted" \
    '{"non_vip_host":true,"gateway_restarted":true}'
}

chaos_worker_kill() {
  local run_id state digest node task container start term_before term_after attempt
  term_before=$(chaos_leader_term)
  for attempt in 1 2 3; do
    run_id=$(chaos_submit)
    read -r node task < <("$docker_bin" service ps "${stack}_backtest" \
      --filter desired-state=running --format '{{.Node}} {{.Name}}' | head -n 1) \
      || chaos_fail "no running backtest task"
    [ -n "$node" ] && [ -n "$task" ] || chaos_fail "no running backtest task"
    container=$(remote "$node" "$docker_bin ps --filter name=$task --format '{{.ID}}'" | head -n 1)
    start=$SECONDS
    chaos_kill_on_host "$node" "$container"
    state=$(chaos_field "$run_id" status)
    chaos_assert_no_partial "$run_id"
    if [ "$state" != "succeeded" ] || [ "$(chaos_field "$run_id" digest)" = "none" ]; then
      break
    fi
  done
  [ "$state" != "succeeded" ] || chaos_fail "the run completed before the worker kill; no in-flight case"
  digest=$(chaos_assert_one_digest "$run_id")
  term_after=$(chaos_leader_term)
  chaos_verify_expected "worker SIGKILL"
  chaos_write HA-18 "Simulated HA" worker-sigkill "$((SECONDS - start))" "$digest" \
    "a backtest task was SIGKILLed; the run was not succeeded without a digest and the expired lease retried it" \
    '{"run_was_in_flight":true,"lease_retried":true,"partial_publish_absent":true}'
  chaos_write HA-20 "Simulated HA" no-partial-publish "$((SECONDS - start))" "$digest" \
    "the killed run exposed no succeeded-without-digest row and every referenced artifact fetched 200" \
    '{"referenced_artifacts_fetched":true,"leader_term_before":"'"$term_before"'","leader_term_after":"'"$term_after"'"}'
  chaos_write HA-21 "Simulated HA" digest-parity "$((SECONDS - start))" "$digest" \
    "tools/verify.sh --expected reproduced the clean baseline after the worker restart" \
    '{"baseline":"'"$chaos_digest"'","observed":"'"${digest#sha256:}"'"}'
}

chaos_leader_kill() {
  local run_id digest node task container host term_before term_after start found state_before attempt
  term_before=$(chaos_leader_term)
  for attempt in 1 2 3; do
    run_id=$(chaos_submit)
    found=0
    state_before=""
    while read -r node task; do
      [ -n "$node" ] || continue
      host=$node
      container=$(remote "$host" "$docker_bin ps --filter name=$task --format '{{.ID}}'" | head -n 1)
      [ -n "$container" ] || continue
      if remote "$host" "$docker_bin exec $container sh -c 'curl -fsS http://127.0.0.1:9100/metrics'" 2>/dev/null \
        | grep -Eq '^quantrule_leader_term\{[^}]*service="evidence"[^}]*\} [1-9][0-9]*$'; then
        state_before=$(chaos_field "$run_id" status)
        start=$SECONDS
        chaos_kill_on_host "$host" "$container"
        found=1
        break
      fi
    done <<< "$("$docker_bin" service ps "${stack}_evidence" --filter desired-state=running --format '{{.Node}} {{.Name}}')"
    [ "$found" -eq 1 ] || chaos_fail "no evidence leader task was found"
    [ "$state_before" != "succeeded" ] && break
  done
  [ "$state_before" != "succeeded" ] || chaos_fail "the run completed before the leader kill; no in-flight case"
  chaos_assert_no_partial "$run_id"
  digest=$(chaos_assert_one_digest "$run_id")
  term_after=$(chaos_leader_term)
  [ "${term_after:-0}" -gt "${term_before:-0}" ] \
    || chaos_fail "the evidence lease term did not increase after the leader kill"
  chaos_verify_expected "evidence-leader SIGKILL"
  chaos_write HA-19 "Simulated HA" evidence-leader-sigkill "$((SECONDS - start))" "$digest" \
    "the evidence leader was SIGKILLed; a higher lease term acquired and the run finalised once" \
    '{"leader_term_before":"'"$term_before"'","leader_term_after":"'"$term_after"'","one_finalisation":true}'
  chaos_write HA-20 "Simulated HA" no-partial-publish-leader "$((SECONDS - start))" "$digest" \
    "the leader-killed run exposed no succeeded-without-digest row and every referenced artifact fetched 200" \
    '{"one_finalisation":true,"referenced_artifacts_fetched":true}'
  chaos_write HA-21 "Simulated HA" digest-parity-leader "$((SECONDS - start))" "$digest" \
    "tools/verify.sh --expected reproduced the clean baseline after the evidence leader restart" \
    '{"baseline":"'"$chaos_digest"'","observed":"'"${digest#sha256:}"'"}'
}

chaos_etcd_health() {
  local host=$1
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin exec -e ETCDCTL_API=3 quantrule-etcd-$host etcdctl \
    --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/quantrule/pki/ca.crt \
    --cert=/etc/quantrule/pki/etcd/client.crt \
    --key=/etc/quantrule/pki/etcd/client.key \
    --dial-timeout=2s --command-timeout=5s endpoint health" >/dev/null 2>&1
}

chaos_etcd_stop_start() {
  local hosts host target start healthy
  hosts=$(jq -r '.stateful.etcd.nodes[]' "$descriptor") || chaos_fail "cannot read etcd nodes"
  [ "$(printf '%s\n' "$hosts" | wc -l)" -ge 3 ] || chaos_fail "the etcd stop case needs three members"
  target=$(printf '%s\n' "$hosts" | tail -n 1)
  start=$SECONDS
  # shellcheck disable=SC2086
  remote "$target" "$docker_bin stop quantrule-etcd-$target" >/dev/null \
    || chaos_fail "cannot stop etcd on $target"
  chaos_stopped+=("$target:quantrule-etcd-$target")
  healthy=0
  for host in $hosts; do
    [ "$host" = "$target" ] && continue
    if chaos_etcd_health "$host"; then healthy=$((healthy + 1)); fi
  done
  [ "$healthy" -eq 2 ] || chaos_fail "the two surviving etcd members are not healthy"
  targeted_check "after etcd member stop"
  # shellcheck disable=SC2086
  remote "$target" "$docker_bin start quantrule-etcd-$target" >/dev/null \
    || chaos_fail "cannot start etcd on $target"
  chaos_stopped=()
  for host in $hosts; do
    chaos_etcd_health "$host" || chaos_fail "etcd $host did not rejoin"
  done
  chaos_write HA-11 "Simulated HA" etcd-member-stop "$((SECONDS - start))" \
    "sha256:$chaos_digest" "one etcd member stopped; two remained healthy and the member rejoined" \
    '{"surviving_members":2,"rejoined":true}'
}

chaos_run() {
  if [ "$skip_chaos" -eq 1 ] || [ "${chaos:-0}" -eq 0 ]; then
    chaos_status="Unverified"
    chaos_say "skipped (--skip-chaos); every chaos row Unverified"
    return 0
  fi
  [ "${confirm:-0}" -eq 1 ] || chaos_fail "--chaos requires --confirm"
  chaos_require_lab
  chaos_shell
  chaos_dataset
  chaos_config
  chaos_pg_host=$(jq -r '.stateful.postgres.nodes[0]' "$descriptor") \
    || chaos_fail "cannot read the first Patroni node"
  trap chaos_cleanup EXIT
  chaos_say "baseline: tools/verify.sh"
  verify_run "chaos baseline"
  chaos_digest=$last_digest
  digest_all "chaos baseline"
  chaos_say "case HA-18/20/21: worker SIGKILL"
  chaos_worker_kill
  chaos_say "case HA-19/20/21: evidence-leader SIGKILL"
  chaos_leader_kill
  chaos_say "case F01: non-VIP gateway SIGKILL"
  chaos_gateway_kill
  chaos_say "case F11: etcd member stop/start"
  chaos_etcd_stop_start
  chaos_write HA-22 "Unverified" real-host-power "0" "Unverified" \
    "operator-only real-ha gate; this script never powers off or resets a host" \
    '{"reason":"no real-host run"}'
  chaos_write HA-23 "Unverified" real-host-az "0" "Unverified" \
    "operator-only real-ha gate; the AZ1 MinIO/ingress limitation stays an accepted risk" \
    '{"reason":"no real-host run"}'
  digest_all "chaos after restore"
  chaos_cleanup
  trap - EXIT
  chaos_status="Simulated HA"
  chaos_say "result: HA-18..HA-21 and F01/F11 Simulated HA; HA-22/HA-23 Unverified; Real HA is not claimed"
}
