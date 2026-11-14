# The full one-command workflow is #644; this target carries reviewer verification.
CONFIG ?= config/examples/verify-mini/run.json
STRATEGY ?= config/examples/verify-mini/strategy.json

.PHONY: verify
verify:
	./tools/verify.sh --config $(CONFIG) --strategy $(STRATEGY)
