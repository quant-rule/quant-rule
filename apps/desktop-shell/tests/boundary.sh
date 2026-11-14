#!/usr/bin/env bash
# Fail when the desktop shell grows a boundary it must not cross.
#
# This is the static half of the smoke test, so it runs on Linux and Windows:
# no IPC command, no local file read in the Rust process, no engine dependency,
# and no Tauri plugin. The dynamic half is tests/smoke-linux.sh.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
status=0

fail() {
  echo "boundary: $1" >&2
  status=1
}

scan() {
  local label="$1"
  local pattern="$2"
  shift 2
  local hits
  hits="$(grep -RInE -- "$pattern" "$@" 2>/dev/null || true)"
  if [[ -n "$hits" ]]; then
    fail "$label"
    printf '%s\n' "$hits" >&2
  fi
}

scan "IPC command" '#\[tauri::command\]|generate_handler!|tauri::ipc|invoke\(' \
  "$root/src" "$root/ui"
scan "local file access in the Rust process" \
  'std::fs|std::path::Path|File::open|read_to_string|tauri_plugin_fs' "$root/src"
scan "Tauri plugin" 'tauri[-_]plugin' "$root/Cargo.toml" "$root/src"
scan "engine import" \
  'quantrule-(data|model|backtest|app|api|server|shell|simulation|research)\b' \
  "$root/Cargo.toml"
scan "workspace path dependency" 'path\s*=\s*"\.\./' "$root/Cargo.toml"

if [[ -d "$root/capabilities" ]] && find "$root/capabilities" -type f | grep -q .; then
  fail "the shell declares a capability file"
fi

if [[ -f "$root/Cargo.lock" ]]; then
  scan "engine in Cargo.lock" \
    'name = "quantrule-(data|model|backtest|app|api|server|shell|simulation|research)"' \
    "$root/Cargo.lock"
fi

if [[ -d "$root/dist" ]]; then
  if find "$root/dist" -type f \( -name '*.csv' -o -name '*.ndjson' -o -name '*.parquet' \) \
    | grep -q .; then
    fail "the staged frontend contains a data file"
  fi
fi

if [[ "$status" -ne 0 ]]; then
  exit 1
fi
echo "boundary ok: no IPC command, no Rust file read, no engine, no plugin"
