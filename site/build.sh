#!/usr/bin/env bash
# Build the Linux bundles, stage them on the static page, and verify the page.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP="$ROOT/apps/desktop"
VERSION="$(python3 - "$DESKTOP/package.json" <<'PY'
import json, sys
print(json.load(open(sys.argv[1]))["version"])
PY
)"

if [ "${1:-}" != "--skip-build" ]; then
  cd "$DESKTOP"
  npm run tauri build
fi

BUNDLE="$DESKTOP/src-tauri/target/release/bundle"
DEB="$BUNDLE/deb/QuantRule_${VERSION}_amd64.deb"
APPIMAGE="$BUNDLE/appimage/QuantRule_${VERSION}_amd64.AppImage"

for artifact in "$DEB" "$APPIMAGE"; do
  if [ ! -f "$artifact" ]; then
    echo "error: missing artifact $artifact" >&2
    exit 1
  fi
done

DEST="$ROOT/site/downloads"
rm -rf "$DEST"
mkdir -p "$DEST"
cp "$DEB" "$DEST/"
cp "$APPIMAGE" "$DEST/"
chmod +x "$DEST/QuantRule_${VERSION}_amd64.AppImage"

cd "$ROOT/site"
sha256sum \
  "downloads/QuantRule_${VERSION}_amd64.deb" \
  "downloads/QuantRule_${VERSION}_amd64.AppImage" > checksums.txt

python3 "$ROOT/site/check.py" --require-artifacts
