# Config examples

`scripts/stage-package.mjs` copies `config/examples/*` here before a bundle, so
the packaged gateway serves the repository example configs from a read-only
resource directory. The supervisor passes this directory as `QR_CONFIG_ROOT`.

The directory is committed with this file so the resource path always resolves.
Staged configs are ignored by Git.
