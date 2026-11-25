# AI Transparency Log — v1.6.0 Licence Decision

- Date: 2026-11-25 (UTC)
- Work item: DOC-004 [#1131](https://github.com/quant-rule/quant-rule/issues/1131)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Resolve the licence contradiction found before the v1.6.0 tag: the support
contract said proprietary, while every crate and both image labels declared MIT
OR Apache-2.0. The human chose MIT OR Apache-2.0. This log records the technical
alignment; commercial support terms remain separate.

## Work performed

- Added the standard `LICENSE-MIT` and `LICENSE-APACHE` texts and a `LICENSE`
  pointer.
- Updated the support contract, the configuration index, the release-readiness
  report, and the v1.6.0 plan to the dual licence.
- Added the changelog entry.

## Verification and evidence

- The 18 crate manifests and both Dockerfiles already carry
  `MIT OR Apache-2.0`; no code or image label changed.
- `qr-governance check --all`: pass.
- The human selected MIT OR Apache-2.0 in the pre-tag question.

## Learning

- A licence boundary must be checked against the crate metadata and image labels
  before a release note is written; the support contract was the outlier.
- Adding the standard texts is a documentation change, not a code change.

## Residual risk

- Commercial support, fees, and hosted-service commitments are not granted by
  the open-source licence; they remain a separate human-approved document.
