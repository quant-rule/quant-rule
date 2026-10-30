#!/usr/bin/env bash
# Run a deterministic experiment matrix with one isolated backtest per cell.
#
# Usage:
#   QR=./target/release/quantrule tools/experiment/run-matrix.sh <manifest> <out-root>
#
# The manifest is a text file with one cell per line:
#   <cell-name> <run.json>
# Blank lines and lines beginning with '#' are ignored. Cell names use letters,
# digits, '.', '-', and '_' only. Cells run in manifest order; the script does
# not read the clock or randomize anything.
set -euo pipefail

usage() {
    printf 'usage: %s <manifest> <out-root>\n' "$0" >&2
}

if [ "$#" -ne 2 ]; then
    usage
    exit 2
fi

manifest=$1
root=$2
qr=${QR:-quantrule}

if [ ! -f "$manifest" ]; then
    printf 'error: manifest not found: %s\n' "$manifest" >&2
    exit 1
fi

if [ -e "$root" ] && [ -n "$(find "$root" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
    printf 'error: output root must be absent or empty: %s\n' "$root" >&2
    exit 1
fi

mkdir -p "$root"
cp "$manifest" "$root/manifest.txt"

count=0
while IFS= read -r line || [ -n "$line" ]; do
    line=${line%$'\r'}
    case "$line" in
        ''|'#'*) continue ;;
    esac

    cell=${line%% *}
    config=${line#* }
    if [ "$cell" = "$line" ] || [ -z "$config" ]; then
        printf 'error: manifest line must be "<cell> <run.json>": %s\n' "$line" >&2
        exit 1
    fi
    if ! [[ "$cell" =~ ^[A-Za-z0-9._-]+$ ]]; then
        printf 'error: unsafe cell name: %s\n' "$cell" >&2
        exit 1
    fi
    if [ ! -f "$config" ]; then
        printf 'error: config not found for %s: %s\n' "$cell" "$config" >&2
        exit 1
    fi

    envelope="$root/.${cell}.backtest.json"
    rm -f "$envelope"
    "$qr" backtest --config "$config" --out "$root/$cell" --json >"$envelope"
    mv "$envelope" "$root/$cell/backtest.json"
    printf 'ran %s\n' "$cell"
    count=$((count + 1))
done < "$manifest"

printf 'matrix %d cells\n' "$count"
