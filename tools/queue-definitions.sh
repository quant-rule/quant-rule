#!/usr/bin/env bash
# Export, verify, import, and retain the RabbitMQ control-plane definitions.
#
# PLAT-004 owns the definitions path, the credential strip, the four HA-005
# queues and their bindings, the no-policy check, and 30-day retention. HA-013
# owns the archive/off-host copy and the empty-broker restore drill. A
# definitions import is a topology restore, not a message restore: a lost queue
# is rebuilt from run and stage rows.
#
# The export runs `rabbitmqctl export_definitions` in a broker container, removes
# `users`, `permissions`, and `topic_permissions` with jq, verifies the topology,
# and stores `<UTC>.json` mode 600 plus a value-free `<UTC>.manifest.json` on
# every broker. No user hash, raw definitions, or secret is printed. The app user
# is recreated from `queue_url` by `tools/stateful.sh queue-bootstrap`.
#
# Usage: tools/queue-definitions.sh [--plan] <command> [options]
#
#   export [--host <broker>]       Export from one broker and store on every broker.
#   verify <file>                  Verify a stored definitions file.
#   verify --live [--host <broker>] Export to a temporary file and verify it.
#   import <file> [--force]        Import into an empty broker/cluster; --force
#                                  overrides a non-empty cluster and never deletes.
#   prune [--dir <dir>] [--now <UTC>] [--days <n>]
#                                  Delete definition files older than 30 days.
#                                  --dir prunes a local directory (test/offline);
#                                  without it every broker's definitions dir is pruned.
#
# Environment:
#   DESCRIPTOR                      descriptor path (default deploy/v3/deploy.example.json)
#   QR_QUEUE_DEFINITIONS_SSH        ssh command and options
#   QR_QUEUE_DEFINITIONS_SUDO       remote privilege command (default "sudo -n"; empty for root)
#   QR_QUEUE_DEFINITIONS_DOCKER     remote docker executable (default docker)
#   QR_QUEUE_DEFINITIONS_DIR        definitions directory (default /srv/quantrule/rabbitmq/definitions)
#   QR_QUEUE_DEFINITIONS_GENERATION queue generation (default 1)
#   QR_QUEUE_DEFINITIONS_DAYS       retention days (default 30)
#   QR_QUEUE_DEFINITIONS_STAMP      fixed export timestamp (test seam)
#
# A live broker loss is lab evidence only in `deploy/v3/queue-failover.sh`;
# this tool never starts, stops, purges, or deletes a queue, user, or vhost.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"

descriptor=${DESCRIPTOR:-deploy/v3/deploy.example.json}
ssh_bin=${QR_QUEUE_DEFINITIONS_SSH:-ssh -o BatchMode=yes -o ConnectTimeout=5}
sudo_bin=${QR_QUEUE_DEFINITIONS_SUDO-sudo -n}
docker_bin=${QR_QUEUE_DEFINITIONS_DOCKER:-docker}
definitions_dir=${QR_QUEUE_DEFINITIONS_DIR:-/srv/quantrule/rabbitmq/definitions}
generation=${QR_QUEUE_DEFINITIONS_GENERATION:-1}
days=${QR_QUEUE_DEFINITIONS_DAYS:-30}
command=""
file=""
host_opt=""
dir_opt=""
now=""
force=0
live=0
plan=0

die() {
  printf 'queue-definitions: %s\n' "$*" >&2
  exit 1
}

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

usage_error() {
  printf 'queue-definitions: %s\n\n' "$*" >&2
  usage >&2
  exit 2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --plan | --dry-run)
      plan=1
      shift
      ;;
    export | import | verify | prune)
      [ -z "$command" ] || usage_error "one command at a time"
      command=$1
      shift
      ;;
    --host)
      [ "$#" -ge 2 ] || usage_error "--host needs a value"
      host_opt=$2
      shift 2
      ;;
    --dir)
      [ "$#" -ge 2 ] || usage_error "--dir needs a value"
      dir_opt=$2
      shift 2
      ;;
    --now)
      [ "$#" -ge 2 ] || usage_error "--now needs a value"
      now=$2
      shift 2
      ;;
    --days)
      [ "$#" -ge 2 ] || usage_error "--days needs a value"
      days=$2
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    --live)
      live=1
      shift
      ;;
    -h | --help | help)
      usage
      exit 0
      ;;
    -*)
      usage_error "unknown argument $1"
      ;;
    *)
      [ -z "$file" ] || usage_error "unexpected argument $1"
      file=$1
      shift
      ;;
  esac
done

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
queue-definitions: plan (no Docker, SSH, jq, or secret file is read)
queue-definitions: descriptor: $descriptor
queue-definitions: 1 export: rabbitmqctl export_definitions in quantrule-rabbitmq-<host>
queue-definitions: 2 strip: jq removes users, permissions, and topic_permissions
queue-definitions: 3 verify: four .g$generation queues, quorum, delivery limit 3, DLX/DLQ, bindings, no runs.*/stages.* policy
queue-definitions: 4 store: $definitions_dir/<UTC>.json mode 600 + <UTC>.manifest.json (no hash or secret) on every broker
queue-definitions: import: only an empty broker/cluster; a live .g generation needs --force; never purge/delete a queue, user, or vhost
queue-definitions: prune: delete definition files older than $days days in $definitions_dir; only timestamped .json and its manifest
queue-definitions: boundary: HA-013 owns archive/off-host copy and the empty-broker restore drill; this tool never touches a message
queue-definitions: no user hash, raw definitions, or secret is printed
PLAN
  exit 0
fi

[ -n "$command" ] || usage_error "a command is required"
case "$command" in
  export)
    [ -z "$file" ] || usage_error "export takes no file"
    [ "$live" -eq 0 ] || usage_error "--live is only valid with verify"
    [ "$force" -eq 0 ] || usage_error "--force is only valid with import"
    [ -z "$dir_opt" ] || usage_error "--dir is only valid with prune"
    [ -z "$now" ] || usage_error "--now is only valid with prune"
    ;;
  import)
    [ -n "$file" ] || usage_error "import needs a definitions file"
    [ "$live" -eq 0 ] || usage_error "--live is only valid with verify"
    [ -z "$dir_opt" ] || usage_error "--dir is only valid with prune"
    [ -z "$now" ] || usage_error "--now is only valid with prune"
    ;;
  verify)
    [ "$force" -eq 0 ] || usage_error "--force is only valid with import"
    [ -z "$dir_opt" ] || usage_error "--dir is only valid with prune"
    [ -z "$now" ] || usage_error "--now is only valid with prune"
    if [ "$live" -eq 1 ]; then
      [ -z "$file" ] || usage_error "verify --live takes no file"
    else
      [ -n "$file" ] || usage_error "verify needs a definitions file or --live"
    fi
    ;;
  prune)
    [ -z "$file" ] || usage_error "prune takes no file"
    [ "$live" -eq 0 ] || usage_error "--live is only valid with verify"
    [ "$force" -eq 0 ] || usage_error "--force is only valid with import"
    ;;
esac
case "$generation" in
  '' | *[!0-9]*) usage_error "QR_QUEUE_DEFINITIONS_GENERATION must be a positive integer" ;;
esac
[ "$generation" -gt 0 ] || usage_error "QR_QUEUE_DEFINITIONS_GENERATION must be positive"
case "$days" in
  '' | *[!0-9]*) usage_error "--days must be a whole number" ;;
esac

command -v jq >/dev/null 2>&1 || die "jq is required for $command"
[ -f "$descriptor" ] || die "descriptor $descriptor is missing"

descriptor_list() {
  jq -r "$1[]" "$descriptor"
}

broker_hosts() {
  descriptor_list '.stateful.rabbitmq.nodes'
}

brokers=$(broker_hosts) || die "cannot read stateful.rabbitmq.nodes"
[ -n "$brokers" ] || die "stateful.rabbitmq.nodes is empty"

select_broker() {
  if [ -n "$host_opt" ]; then
    printf '%s\n' "$brokers" | grep -Fxq "$host_opt" \
      || die "$host_opt is not in stateful.rabbitmq.nodes"
    printf '%s' "$host_opt"
  else
    printf '%s\n' "$brokers" | head -n 1
  fi
}

container_name() {
  printf 'quantrule-rabbitmq-%s' "$1"
}

remote() {
  local host=$1
  shift
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$@"
}

rabbitmq_exec() {
  local host=$1
  shift
  local container
  container=$(container_name "$host")
  # shellcheck disable=SC2086
  remote "$host" "$sudo_bin $docker_bin exec -u rabbitmq $container $*"
}

safe_component() {
  case "$1" in
    '' | *[!0-9A-Za-z_.-]*) die "unsafe path component $1" ;;
  esac
}

safe_directory() {
  case "$1" in
    /*) ;;
    *) die "the definitions directory must be absolute: $1" ;;
  esac
  case "$1" in
    *[!0-9A-Za-z_./-]*) die "unsafe definitions directory $1" ;;
  esac
  case "/$1/" in
    *"/../"*) die "the definitions directory must not contain ..: $1" ;;
  esac
}

capture_script='set -eu
umask 077
tmp=$(mktemp /tmp/qr-definitions.XXXXXX)
rabbitmqctl export_definitions "$tmp" >/dev/null
cat "$tmp"
rm -f "$tmp"'

import_script='set -eu
umask 077
tmp=$(mktemp /tmp/qr-definitions.XXXXXX)
cat > "$tmp"
rabbitmqctl import_definitions "$tmp" >/dev/null
rm -f "$tmp"'

copy_script='set -eu
umask 077
dir=$1
name=$2
mode=$3
[ -d "$dir" ] || { printf "queue-definitions: missing %s; run tools/stateful.sh copy/start first\n" "$dir" >&2; exit 3; }
cat > "$dir/$name"
chmod "$mode" "$dir/$name"
chown --reference="$dir" "$dir/$name"'

capture() {
  local host=$1 dest=$2
  safe_directory "$definitions_dir"
  # shellcheck disable=SC2086
  remote "$host" "$sudo_bin $docker_bin exec -u rabbitmq $(container_name "$host") sh -c '$capture_script'" \
    > "$dest" || die "cannot export definitions from $host"
}

strip() {
  local raw=$1 clean=$2
  jq -e 'del(.users, .permissions, .topic_permissions)
         | if has("users") or has("permissions") or has("topic_permissions")
           then error("credential blocks remain")
           else . end' "$raw" > "$clean" \
    || die "cannot strip users and permissions from the export"
}

copy_one() {
  local host=$1 source=$2 directory=$3 name=$4 mode=$5
  safe_directory "$directory"
  safe_component "$name"
  # shellcheck disable=SC2086
  remote "$host" "$sudo_bin sh -c '$copy_script' -- '$directory' '$name' '$mode'" \
    < "$source" || die "cannot store $name on $host"
}

verify_file() {
  local path=$1
  local backtest="runs.backtest.g$generation"
  local evidence="runs.evidence.g$generation"
  local dead="runs.dead-letter.g$generation"
  local stage="stages.worker.g$generation"
  local failures=()
  [ -f "$path" ] || die "definitions file $path is missing"
  jq -e . "$path" >/dev/null 2>&1 || die "definitions file $path is not JSON"
  if grep -q 'password_hash' "$path"; then
    failures+=("a password_hash key is present")
  fi
  if ! jq -e '(.users? // []) | length == 0' "$path" >/dev/null 2>&1; then
    failures+=("users are still present; strip them before storage")
  fi
  if ! jq -e '(.permissions? // []) | length == 0' "$path" >/dev/null 2>&1; then
    failures+=("permissions are still present; strip them before storage")
  fi
  if ! jq -e '(.topic_permissions? // []) | length == 0' "$path" >/dev/null 2>&1; then
    failures+=("topic_permissions are still present; strip them before storage")
  fi
  local queue
  for queue in "$backtest" "$evidence" "$dead" "$stage"; do
    if ! jq -e --arg name "$queue" \
      '[.queues[]? | select(.name == $name and .vhost == "/")] | length == 1' \
      "$path" >/dev/null 2>&1; then
      failures+=("queue $queue is missing from the default vhost")
      continue
    fi
    if ! jq -e --arg name "$queue" \
      '[.queues[]? | select(.name == $name and .vhost == "/" and .durable == true)] | length == 1' \
      "$path" >/dev/null 2>&1; then
      failures+=("queue $queue is not durable")
    fi
    if ! jq -e --arg name "$queue" \
      '[.queues[]? | select(.name == $name and .vhost == "/" and (((.arguments // {})["x-queue-type"]) == "quorum"))] | length == 1' \
      "$path" >/dev/null 2>&1; then
      failures+=("queue $queue is not quorum")
    fi
  done
  for queue in "$backtest" "$evidence" "$stage"; do
    if ! jq -e --arg name "$queue" \
      '[.queues[]? | select(.name == $name and .vhost == "/" and (((.arguments // {})["x-delivery-limit"] | tostring) == "3"))] | length == 1' \
      "$path" >/dev/null 2>&1; then
      failures+=("queue $queue lacks x-delivery-limit 3")
    fi
    if ! jq -e --arg name "$queue" \
      '[.queues[]? | select(.name == $name and .vhost == "/" and (((.arguments // {})["x-dead-letter-exchange"]) == "quantrule.runs.dlx"))] | length == 1' \
      "$path" >/dev/null 2>&1; then
      failures+=("queue $queue lacks the quantrule.runs.dlx dead-letter exchange")
    fi
  done
  if ! jq -e --arg name "$dead" \
    '[.queues[]? | select(.name == $name and .vhost == "/" and (((.arguments // {}) | has("x-dead-letter-exchange")) or ((.arguments // {}) | has("x-delivery-limit"))))] | length == 0' \
    "$path" >/dev/null 2>&1; then
    failures+=("the dead-letter queue $dead has a dead-letter target or delivery limit")
  fi
  local exchange
  for exchange in quantrule.runs quantrule.runs.dlx; do
    if ! jq -e --arg name "$exchange" \
      '[.exchanges[]? | select(.name == $name and .vhost == "/" and .type == "direct" and .durable == true)] | length == 1' \
      "$path" >/dev/null 2>&1; then
      failures+=("exchange $exchange is missing, not direct, or not durable")
    fi
  done
  check_binding() {
    local source=$1 destination=$2 key=$3
    jq -e --arg source "$source" --arg destination "$destination" --arg key "$key" \
      'any(.bindings[]?; .source == $source and .destination == $destination
           and .destination_type == "queue" and .routing_key == $key and .vhost == "/")' \
      "$path" >/dev/null 2>&1 \
      || failures+=("binding $source -> $destination ($key) is missing")
  }
  check_binding quantrule.runs "$backtest" run.requested
  check_binding quantrule.runs "$backtest" run.cancelled
  check_binding quantrule.runs "$evidence" run.computed
  check_binding quantrule.runs "$dead" run.dead
  check_binding quantrule.runs "$stage" stage.requested
  check_binding quantrule.runs "$stage" stage.cancelled
  for key in run.requested run.cancelled run.computed stage.requested stage.cancelled; do
    check_binding quantrule.runs.dlx "$dead" "$key"
  done
  if ! jq -e \
    '[.policies[]? as $policy
      | select((("runs.backtest.g1" | test($policy.pattern // ""))
                or ("stages.worker.g1" | test($policy.pattern // ""))
                or (($policy.name // "") | test("^(runs|stages)\\."))))]
     | length == 0' "$path" >/dev/null 2>&1; then
    failures+=("a policy targets runs.* or stages.*")
  fi
  if [ "${#failures[@]}" -gt 0 ]; then
    printf 'queue-definitions: verify failed for %s:\n' "$path" >&2
    printf '  - %s\n' "${failures[@]}" >&2
    return 1
  fi
  printf 'queue-definitions: verified %s: %s, %s, %s, %s; quorum, delivery limit 3, DLX/DLQ, required bindings, no runs.*/stages.* policy\n' \
    "$path" "$backtest" "$evidence" "$dead" "$stage" >&2
}

export_definitions() {
  local source stamp raw clean manifest name manifest_name
  source=$(select_broker)
  stamp=${QR_QUEUE_DEFINITIONS_STAMP:-$(date -u +%Y-%m-%dT%H%M%SZ)}
  case "$stamp" in
    *[!0-9A-Za-z.-]*) die "the export timestamp contains an unsafe character" ;;
  esac
  name="$stamp.json"
  manifest_name="$stamp.manifest.json"
  local work
  work=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-definitions.XXXXXX")
  raw="$work/raw.json"
  clean="$work/definitions.json"
  manifest="$work/manifest.json"
  trap "rm -f '$raw' '$clean' '$manifest'; rmdir '$work' 2>/dev/null || true" EXIT
  capture "$source" "$raw"
  strip "$raw" "$clean"
  verify_file "$clean"
  local queues_json exchanges_json bytes
  queues_json=$(jq -c '[.queues[]?.name] | sort' "$clean") || die "cannot list queues"
  exchanges_json=$(jq -c '[.exchanges[]?.name] | sort' "$clean") || die "cannot list exchanges"
  bytes=$(wc -c < "$clean" | tr -d ' ')
  jq -n \
    --arg exported_at "$stamp" \
    --argjson generation "$generation" \
    --arg definitions "$name" \
    --argjson bytes "$bytes" \
    --argjson queues "$queues_json" \
    --argjson exchanges "$exchanges_json" \
    --argjson retention_days "$days" \
    '{
      v: "quantrule.queue-definitions/v1",
      exported_at: $exported_at,
      generation: $generation,
      definitions: $definitions,
      bytes: $bytes,
      queues: $queues,
      exchanges: $exchanges,
      credentials: "users, permissions, and topic_permissions stripped",
      retention_days: $retention_days,
      note: "HA-013 owns archive/off-host copy and the empty-broker restore drill; no hash or secret is recorded"
    }' > "$manifest" || die "cannot write the definitions manifest"
  local host
  for host in $brokers; do
    copy_one "$host" "$clean" "$definitions_dir" "$name" 600
    copy_one "$host" "$manifest" "$definitions_dir" "$manifest_name" 644
    printf 'queue-definitions: stored %s on %s\n' "$name" "$host"
  done
  printf 'queue-definitions: exported %s from %s (%s queues, %s exchanges); users/permissions stripped\n' \
    "$name" "$source" "$(jq '.queues | length' "$clean")" "$(jq '.exchanges | length' "$clean")"
}

verify_command() {
  if [ "$live" -eq 1 ]; then
    local source work raw clean
    source=$(select_broker)
    work=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-definitions.XXXXXX")
    raw="$work/raw.json"
    clean="$work/definitions.json"
    trap "rm -f '$raw' '$clean'; rmdir '$work' 2>/dev/null || true" EXIT
    capture "$source" "$raw"
    strip "$raw" "$clean"
    verify_file "$clean"
  else
    verify_file "$file"
  fi
}

import_definitions() {
  local target
  target=$(select_broker)
  verify_file "$file"
  rabbitmq_exec "$target" rabbitmq-diagnostics -q ping >/dev/null \
    || die "RabbitMQ on $target does not answer rabbitmq-diagnostics -q ping"
  local queues
  queues=$(rabbitmq_exec "$target" rabbitmqctl -q list_queues -p / name) \
    || die "cannot list the queues on $target"
  queues=$(printf '%s\n' "$queues" | sed '/^[[:space:]]*$/d')
  if [ -n "$queues" ]; then
    if [ "$force" -eq 0 ]; then
      if printf '%s\n' "$queues" | grep -Eq '\.g[0-9]+$'; then
        die "the cluster has a live queue generation; pass --force to import over it"
      fi
      die "the cluster is not empty; import is allowed only into an empty broker/cluster"
    fi
    printf 'queue-definitions: --force: importing over a non-empty cluster; no queue, user, or vhost is deleted\n' >&2
  fi
  # shellcheck disable=SC2086
  remote "$target" "$sudo_bin $docker_bin exec -i -u rabbitmq $(container_name "$target") sh -c '$import_script'" \
    < "$file" || die "cannot import $file on $target"
  verify_live "$target"
  printf 'queue-definitions: imported %s on %s; verified live topology\n' "$file" "$target"
}

verify_live() {
  local target=$1 work raw clean
  work=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-definitions.XXXXXX")
  raw="$work/raw.json"
  clean="$work/definitions.json"
  trap "rm -f '$raw' '$clean'; rmdir '$work' 2>/dev/null || true" EXIT
  capture "$target" "$raw"
  strip "$raw" "$clean"
  verify_file "$clean"
}

prune_local() {
  local directory=$1 cutoff=$2 count=0 path name stamp
  for path in "$directory"/*.json; do
    [ -f "$path" ] || continue
    name=${path##*/}
    case "$name" in
      *.manifest.json) continue ;;
    esac
    stamp=${name%.json}
    [[ "$stamp" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{6}Z$ ]] || continue
    if [[ "$stamp" < "$cutoff" ]]; then
      rm -f -- "$path"
      rm -f -- "$directory/$stamp.manifest.json"
      count=$((count + 1))
    fi
  done
  printf 'queue-definitions: pruned %s definition file(s) older than %s in %s\n' \
    "$count" "$cutoff" "$directory"
}

prune() {
  local cutoff now_seconds
  local reference=${now:-$(date -u +%Y-%m-%dT%H%M%SZ)}
  [[ "$reference" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{6}Z$ ]] \
    || die "--now must be YYYY-MM-DDTHHMMSSZ"
  local reference_date=${reference:0:10}
  local reference_time="${reference:11:2}:${reference:13:2}:${reference:15:2}"
  now_seconds=$(date -u -d "$reference_date $reference_time" +%s) \
    || die "cannot parse $reference with date"
  cutoff=$(date -u -d "@$((now_seconds - days * 86400))" +%Y-%m-%dT%H%M%SZ) \
    || die "cannot compute the retention cutoff"
  if [ -n "$dir_opt" ]; then
    safe_directory "$dir_opt"
    [ -d "$dir_opt" ] || die "definitions directory $dir_opt is missing"
    prune_local "$dir_opt" "$cutoff"
    return 0
  fi
  local host name stamp count=0
  for host in $brokers; do
    safe_directory "$definitions_dir"
    local names
    names=$(remote "$host" "$sudo_bin ls -1 '$definitions_dir'") \
      || die "cannot list $definitions_dir on $host"
    while IFS= read -r name; do
      [ -n "$name" ] || continue
      case "$name" in
        *.manifest.json) continue ;;
      esac
      stamp=${name%.json}
      [[ "$stamp" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{6}Z$ ]] || continue
      safe_component "$name"
      if [[ "$stamp" < "$cutoff" ]]; then
        remote "$host" "$sudo_bin rm -f -- '$definitions_dir/$name'" \
          || die "cannot remove $name on $host"
        remote "$host" "$sudo_bin rm -f -- '$definitions_dir/$stamp.manifest.json'" \
          || true
        count=$((count + 1))
      fi
    done <<< "$names"
  done
  printf 'queue-definitions: pruned %s definition file(s) older than %s\n' "$count" "$cutoff"
}

case "$command" in
  export) export_definitions ;;
  verify) verify_command ;;
  import) import_definitions ;;
  prune) prune ;;
  *) die "unknown command $command" ;;
esac
