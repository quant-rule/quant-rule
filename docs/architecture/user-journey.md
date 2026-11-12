# User Journey

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Claim type: Outcome Claim

The single authority for how the product is used and what it returns. Every other
document links here instead of restating it.

## 1. One host

| # | The operator does | Sees | Value |
|---|---|---|---|
| 1 | `make up` | `ready → http://localhost:8080` | one command, no configuration file |
| 2 | Opens the URL | `connected · v1.0.0` | a client that is only a browser |
| 3 | Uploads a candle CSV | `data_version`, candles, coverage | data identified by content |
| 4 | Picks a config, sees `valid`, submits | `202`, run appears `queued` | submission returns immediately |
| 5 | Closes the laptop | — | the run does not depend on the session |
| 6 | Returns and opens the URL | `succeeded`, stages, provenance, digest, artifacts | evidence with an audit trail |
| 7 | Copies the digest, or runs `make verify` | one hash | reproducible, comparable, defensible |

## 2. Client on a laptop, compute on a server

| # | The operator does | Value |
|---|---|---|
| 1 | Starts the same stack on the server with `make remote` | one deployment, two machines |
| 2 | Points the browser at `http://server:8080` | no install on the laptop |
| 3 | Uploads the dataset once | the bytes cross the network once, not per run |
| 4 | Submits, closes the laptop | the server continues |
| 5 | Returns from any machine | the run list is server state |
| 6 | Downloads the bundle or compares the digest | portable evidence |

## 3. Reviewer reproduction

| # | The reviewer does | Expects |
|---|---|---|
| 1 | `make up` | healthy stack |
| 2 | `make verify` | a canonical digest |
| 3 | Compares with the recorded digest | identical, or a recorded failure |

Four commands, one line of output. The reviewer does not need an account, a
credential, or access to the operator's machine.

## 4. What the product is worth

| Value | Because |
|---|---|
| **Reproducible** | The same inputs and image digest produce the same data artifacts |
| **Auditable** | Every result names its data version, config hash, model version, and image digest |
| **Detachable** | A run outlives the client that submitted it |
| **Portable** | Any machine with the URL uses it; the compute can move |
| **Sovereign** | No account, no vendor; candle data stays on the operator's network |
| **Honest** | A negative result is valid and the workflow can stop |

## 5. What the operator never does

- Type a filesystem path.
- Create an account.
- Install a desktop application.
- Compare a bundle by hand.
- Wait with a terminal open.
