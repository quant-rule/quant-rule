# Sidecars

`scripts/stage-package.mjs` copies the six service binaries and `qr-desktop`
into this directory before a bundle. Tauri copies it to `<resource>/sidecars`,
and the shell passes the resource directory to the supervisor.

The directory is committed with this file so a development `cargo build` and
`tauri build` still resolve the resource path; a bundle built without staging
contains no sidecars, and `tests/check-package.sh` fails it. Staged binaries are
ignored by Git.
