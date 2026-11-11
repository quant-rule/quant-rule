# QuantRule Linux Downloads

- Status: Current; Phase 3.5
- Page: `site/index.html`
- Builder: `site/build.sh`
- Checker: `site/check.py`

The page is static HTML/CSS. It has no JavaScript, no analytics, no account, and
no backend. The artifacts and checksums are release assets; `site/downloads/` and
`site/checksums.txt` are generated and not committed.

## Build and stage

```bash
cd apps/desktop
npm ci
cd ../..
./site/build.sh
```

The script runs `npm run tauri build`, copies the `.deb` and `.AppImage` into
`site/downloads/`, writes `site/checksums.txt`, and runs the checker with
`--require-artifacts`.

## Check without artifacts

```bash
python3 site/check.py
```

This checks the page links, version, and boundary text. It skips the artifact
hash check when the downloads are absent.

## Serve locally

```bash
python3 -m http.server 8080 --directory site
```

Open `http://127.0.0.1:8080/` and download both artifacts. Verify with:

```bash
cd site
sha256sum -c checksums.txt
```

## Deploy

Upload `site/index.html`, `site/styles.css`, `site/downloads/`, and
`site/checksums.txt` to any static host, or attach the two artifacts and
`checksums.txt` to the GitHub release and point the page links at the release
assets. The page itself does not need a runtime.

The desktop app remains local: the download page is distribution only. No
account, cloud compute, or cloud data store is part of this release.
