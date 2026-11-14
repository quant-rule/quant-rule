# The QuantRule operator interface: one command starts the stack.
#
# `make up` is the Linux and macOS path. Windows runs compose.yaml directly with
# `docker compose up -d --build --wait`; make is optional there. Fedora may use
# Podman through `make up COMPOSE="podman compose"`.
#
# This file forwards to Docker Compose and the repository scripts only. It holds
# no domain logic, and it is not part of the product runtime.

.DEFAULT_GOAL := help
SHELL := /bin/sh

COMPOSE ?= docker compose
QR_URL ?= http://localhost:8080
CONFIG ?= config/examples/verify-mini/run.json
STRATEGY ?= config/examples/verify-mini/strategy.json

.PHONY: help up down reset logs ps scale gate verify test test-fast test-composed test-parity remote token backup restore dev-web

help: ## Show this help
	@awk 'BEGIN {FS = ":.*## "; printf "QuantRule make targets\n\n"} /^[a-zA-Z0-9_-]+:.*## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@printf '\nWindows: docker compose up -d (make is optional).\n'
	@printf 'Fedora: make up COMPOSE="podman compose".\n'

up: ## Build and start the default stack, wait for health, print the URL
	@command -v curl >/dev/null || { echo "up: curl is required to check /health" >&2; exit 1; }
	$(COMPOSE) up -d --build --wait
	@curl -fsS "$(QR_URL)/health" >/dev/null
	@version="$$(curl -fsS "$(QR_URL)/version" | sed -n 's/.*"product":"\([^"]*\)".*/\1/p')"; \
	if [ -z "$$version" ]; then \
		echo "up: the gateway did not report a product version" >&2; \
		exit 1; \
	fi; \
	printf 'QuantRule v%s ready → %s\n' "$$version" "$(QR_URL)"

down: ## Stop the stack, keep volumes
	$(COMPOSE) down

reset: ## Stop and delete all volumes, after confirmation
	@printf 'This deletes every QuantRule volume. Continue? [y/N] '; \
	read -r answer || answer=""; \
	case "$$answer" in [yY]|[yY][eE][sS]) $(COMPOSE) down -v ;; *) echo "reset: cancelled"; exit 1 ;; esac

logs: ## Follow the stack logs
	$(COMPOSE) logs -f --tail=200

ps: ## Show container status
	$(COMPOSE) ps

scale: ## Scale model and backtest (make scale N=4)
	@case "$(N)" in \
		''|*[!0-9]*) echo "scale: set N to a whole number, for example make scale N=4" >&2; exit 2 ;; \
	esac
	@active="$$($(COMPOSE) config --services)" || { echo "scale: cannot read the Compose service list" >&2; exit 1; }; \
	if printf '%s\n' "$$active" | grep -qx model; then \
		$(COMPOSE) up -d --no-deps --scale model=$(N) --scale backtest=$(N) model backtest; \
	else \
		echo "scale: the six-binaries profile is not available yet; qr-model and qr-data do not exist (#643)." >&2; \
		echo "scale: nothing was scaled. When the profile is enabled, this target scales model and backtest to N." >&2; \
		exit 1; \
	fi

gate: ## Run the full workspace gate
	./tools/gate.sh

verify: ## Run the fixture and print the canonical digest
	./tools/verify.sh --config $(CONFIG) --strategy $(STRATEGY)

test: test-fast ## Run the fast unit and integration tier
test-fast: ## Run the fast unit and integration tier
	./tools/test_tiers.sh fast
test-composed: ## Run the composed tier against the real stack
	./tools/test_tiers.sh composed
test-parity: ## Run the port-parity tier
	./tools/test_tiers.sh parity

remote: ## Start with only :8080 published and a bearer token
	@if [ ! -f tools/remote.sh ]; then echo "remote: tools/remote.sh is not in this tree yet; it lands with DIST-081 (#672)." >&2; exit 1; fi
	./tools/remote.sh

token: ## Print a new bearer token
	@if [ ! -f tools/token.sh ]; then echo "token: tools/token.sh is not in this tree yet; it lands with DIST-081 (#672)." >&2; exit 1; fi
	./tools/token.sh

backup: ## Dump Postgres and mirror the object store
	@if [ ! -f tools/backup.sh ]; then echo "backup: tools/backup.sh is not in this tree yet; it lands with DIST-080 (#671)." >&2; exit 1; fi
	./tools/backup.sh

restore: ## Restore an archive into a clean stack (make restore FILE=<archive>)
	@if [ ! -f tools/restore.sh ]; then echo "restore: tools/restore.sh is not in this tree yet; it lands with DIST-080 (#671)." >&2; exit 1; fi
	@if [ -z "$(FILE)" ]; then echo "restore: set FILE to the archive path, for example make restore FILE=backup.tar.gz" >&2; exit 2; fi
	./tools/restore.sh "$(FILE)"

dev-web: ## Run the web client dev server against localhost:8080
	@command -v bun >/dev/null || { echo "dev-web: Bun is required to run the web client (https://bun.sh)." >&2; exit 1; }
	@if [ ! -d apps/web/node_modules ]; then (cd apps/web && bun install --frozen-lockfile); fi
	cd apps/web && bun run dev
