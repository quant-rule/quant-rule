# Improvement run index

Runs: 2.

| run id | date | tier | hypothesis | changed | frozen | baseline | partition | verdict | artifacts |
|---|---|---|---|---|---|---|---|---|---|
| MODEL-2026-11-03-01 | 2026-11-03 | model | At least one signal-role model state or event has a stable forward edge on the model_dev partition. | model | strategy, portfolio |  | model_dev | not green — no signal-role family passed | 3 |
| MODEL-2026-11-03-02 | 2026-11-03 | model | The model_dev gate result replicates on the one-shot model_validation partition. | model | strategy, portfolio |  | model_validation | not green — validation-only B; combined gate no edge | 3 |
