# QuantRule Desktop

- Status: Current; Phase 3.4
- Stack: Tauri 2, Svelte 5, TypeScript, Vite
- Boundary: the UI calls only `quantrule-app`; it does not spawn the CLI.

## What it does

| Section | Capability |
|---|---|
| Workspace | Open a local root and list `config/` runs and `data/sources/` datasets. |
| Data | Read one M1 CSV and show its candle count and open-time extent. |
| Configs | Validate a run and show its data, model, execution, and portfolio composition. |
| Run | Compute the model over the configured ladder and export the run summary. |
| Memory | Open the SQLite memory, record a run, view metrics/claims/notes, export/import, backup/restore. |
| Settings | Set the workspace root and memory database path. |

The desktop UI is a first-class surface, equal to the shell. The pipeline stages
(backtest, strategy, execution, portfolio, qualification, analytics, and the
stage reports) are being moved behind the shared application service under the
[audit remediation plan](../../docs/execution-plans/2026-11-11/audit-app-service-native-remediation.md).
Until that phase lands, those stages remain shell-only and are a recorded parity
gap.

## Develop

```bash
cd apps/desktop
npm ci
npm run dev
```

The Vite page uses a deterministic fixture backend when Tauri is absent or
`VITE_MOCK_IPC=1` is set. The native app uses the Tauri IPC path.

## Verify

```bash
npm run check
npm test
npm run test:e2e
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --no-bundle
xvfb-run -a timeout 15 ./src-tauri/target/release/quantrule-desktop
```

The last command exits `124` when the window stays open until the timeout; that
is the expected smoke result.

## Native dependencies

Fedora:

```bash
sudo dnf install -y gtk3-devel webkit2gtk4.1-devel libsoup3-devel \
  openssl-devel libappindicator-gtk3-devel librsvg2-devel \
  librsvg2-tools patchelf xorg-x11-server-Xvfb
```

The Tauri crate is a standalone workspace under `src-tauri`; the root Cargo
workspace excludes it so the engine gate does not require GUI libraries.
