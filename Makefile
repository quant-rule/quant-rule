# The full one-command workflow is #644; this target carries reviewer verification.
CONFIG ?= config/examples/verify-mini/run.json
STRATEGY ?= config/examples/verify-mini/strategy.json

.PHONY: verify test test-fast test-composed test-parity
verify:
	./tools/verify.sh --config $(CONFIG) --strategy $(STRATEGY)

# The three test tiers from AGENTS.md section 4 and ADR-13.
test: test-fast
test-fast:
	./tools/test_tiers.sh fast
test-composed:
	./tools/test_tiers.sh composed
test-parity:
	./tools/test_tiers.sh parity
