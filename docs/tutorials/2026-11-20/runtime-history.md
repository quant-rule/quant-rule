# Runtime History

- Status: Current
- Date: 2026-11-20
- Product: shell and browser
- Depends on: #937 runtime history

Runtime history is the product surface for list, archive, and erase of the
records the services own. Repository evidence is not here; it stays in
`qr-governance history`.

## List

`history list` names one kind: `run`, `stage`, `research`, `simulation`, or
`dataset`. Archived records are hidden unless `--include-archived` is passed.

```text
quantrule history list --kind run --include-archived --limit 20
quantrule history list --kind research
quantrule history list --kind dataset
```

## Archive

Archive hides a record from the default list and keeps its bytes.

```text
quantrule history archive --kind run --id sha256:1111111111111111111111111111111111111111111111111111111111111111
quantrule history archive --kind research --id MODEL-2026-11-20-01
quantrule history archive --kind dataset --id sha256:2222222222222222222222222222222222222222222222222222222222222222
```

`--dry-run` prints the intent and calls nothing.

## Erase

Erase requires the record to be archived and the exact id as confirmation.
Succeeded runs, decided research records, checked or decided simulation runs,
and datasets with any registered reference refuse erase.

```text
quantrule history erase --kind run --id sha256:1111111111111111111111111111111111111111111111111111111111111111 --confirm sha256:1111111111111111111111111111111111111111111111111111111111111111
quantrule history erase --kind stage --id sha256:3333333333333333333333333333333333333333333333333333333333333333 --confirm sha256:3333333333333333333333333333333333333333333333333333333333333333
```

The browser History screen exposes the same kinds, the same filters, and the same
confirmation rule.
