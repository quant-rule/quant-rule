# Web Client

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-05`](../adr/2026-11-12/05-svelte-only-web-client.md)
- Claim type: Behaviour Claim

The client is a Svelte 5 single-page application served by the gateway at the
same origin as the API. It holds no domain logic and no source of truth.

## 1. Screens

| Screen | Shows | Empty state |
|---|---|---|
| Connection | Mode, URL, health, version, image digest | `unreachable` with a retry |
| Datasets | `data_version`, candles, first and last bar, upload | "No datasets. Upload a CSV." |
| Configs | Name, origin (repo or uploaded), validity, parser message, submit | "No configs on the server." |
| Runs | Status, id, config, data version, submitted, duration, cancel | "No runs. Submit one from Configs." |
| Run detail | Stages, provenance, artifacts, digest, result | Only for a known run |
| Memory | Runs, metrics, claims, notes | "Nothing recorded yet." |
| Settings | Connection, upload limit, workspace | — |

## 2. Stores

| Store | Holds | Polling |
|---|---|---|
| `transport.svelte.ts` | mode, URL, health, version | 30 s idle |
| `runs.svelte.ts` | list, selected run, polling state | 2 s active, 30 s idle |
| `catalog.svelte.ts` | datasets, configs, artifacts | on demand |
| `labels.generated.ts` | numeric code → word | static, generated |

Each store has one server resource. No store caches a result for offline use.

## 3. Transport

One function keeps the existing shape:

```ts
call<T>(path: string, body?: unknown): Promise<T>
```

- Production: `fetch` against `/api/v1`.
- Test: an HTTP fixture router keyed by path and method; no socket.
- The mock backend is the only test seam and must not diverge from the contract.

## 4. Disconnection

When the gateway is unreachable:

- The connection pill turns red.
- The runs list shows `disconnected`, not stale rows.
- Submission is disabled.
- No result is read from local storage.

A tool whose value is "these bytes are the truth" must not present stale
evidence. Only connection settings persist locally.

## 5. Numbers, not labels

The gateway returns codes. The client maps them using
`labels.generated.ts`, produced from the Rust registry and checked by the gate.
A code with no mapping renders as `unknown(<code>)`, never as a guess.

## 6. Config origin

The list merges the read-only repository mount with uploaded configs. The origin
is shown as a label, never as a path. A repository config updates when the file
changes and the data service reloads; it is not editable in the client.

## 7. Upload

The client streams the CSV to `POST /catalog/datasets` and displays progress.
The server owns the limit, the hash, and the catalog row. The client never
constructs a `data_version`.

## 7. Accessibility and determinism

- Every action is reachable by keyboard and announced by role.
- Status is text plus colour, never colour alone.
- Rendered order is fixed; no timestamp is rendered from the client clock.
- The same fixture produces the same DOM in Playwright.

## 8. Out of scope

No offline mode, no native shell, no local file access beyond upload, no charting
library, no client-side computation of a model value.
