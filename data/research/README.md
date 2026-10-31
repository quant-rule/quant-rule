# Research

Output of `research export`.

- Layout: `raw/<dataset>/m1.csv` and `processed/<dataset>/` with `layer1.csv`,
  `layer2.csv`, `layer3.csv`, `manifest.json`, `data_dictionary.csv`,
  `data_dictionary.md`, `codes.csv`, and `README.md`.
- Produced by: `research export <csv> <dataset> --data-version <id>`.
- Input is capped at 100,000 M1 candles; an over-cap input fails rather than
  being truncated.
- The export is deterministic; regenerate it instead of copying it.
- Ordinal codes stay counts; the layers never average axes together.
