# Quant Rule — browser client

Svelte 5 single-page application. It talks to the gateway over
`quantrule.api/v1` and holds no domain logic.

## Run

```bash
bun install
bun run dev      # proxies /api to localhost:8080
```

## Check

```bash
bun run check    # svelte-check
bun test         # unit tests
bun run build    # static assets in dist/
```

The built assets are served by the gateway at the same origin as the API; no
Bun runtime exists in the image.

## Layout

| Path | Role |
|---|---|
| `src/App.svelte` | Shell and navigation |
| `src/lib/api.ts` | The only transport seam |
| `src/lib/types.ts` | Contract types |
| `src/lib/connection.svelte.ts` | Gateway state |
| `src/lib/ConnectionPanel.svelte` | Connection screen |
| `src/lib/datasets.svelte.ts` | Dataset list and upload state |
| `src/lib/DatasetsPanel.svelte` | Datasets screen and CSV upload |
| `src/lib/profiles.ts` | Active-profile selection |
| `src/lib/profiles.svelte.ts` | Profile list, create, and remembered id |
| `src/lib/ProfilesPanel.svelte` | Profiles screen |
| `src/lib/ProfileSwitcher.svelte` | Active profile in the topbar |

Screens arrive with their issues; the connection and datasets screens are
implemented.
