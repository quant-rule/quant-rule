# Reports

Descriptive output from `market-report` and standalone `report <engine>` runs.

- `market-report` writes exactly `summary.txt`, `report.json`,
  `price-axes.png`, and `events.png` into one directory.
- `report <engine>` writes `<out>/<engine>/` with that engine's bundle.
- The target directory must be absent or empty; a run never mixes with an
  earlier one.
- One directory per labelled run; include the data version and date in the
  label when more than one instrument is in use.
- Reports contain no order, fill, position, or P&L field.
