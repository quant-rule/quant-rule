# Labs

One complete `backtest` result plus its root summary.

- Naming: `lab-result-YYYY-MM-DDTHH-MM-SSZ` in UTC.
- Produced by: `backtest --out data/labs/<lab>` followed by
  `lab summary <lab> --role ... --created-utc ...`.
- Content: the five bundles `model`, `strategy`, `execution`, `portfolio`, and
  `analytical`, plus one `summary.txt`.
- A lab is evidence only for the data version and configuration recorded in it.
- Delete a lab when it is no longer cited; a stale lab is misleading.
