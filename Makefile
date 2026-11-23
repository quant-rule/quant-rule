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
REPLICA ?= config/examples/verify-mini-replica/run.json
SECRETS_DIR ?= secrets
PKI_DIR ?= pki
COMPOSE_ENV = QR_SECRETS_DIR="$(abspath $(SECRETS_DIR))" QR_PKI_DIR="$(abspath $(PKI_DIR))" $(COMPOSE) --env-file "$(abspath $(SECRETS_DIR))/compose.env"

.PHONY: help secrets pki up down reset logs ps scale gate verify test test-fast test-composed test-parity test-ha remote token compose-check backup restore dev-web swarm-validate swarm-render swarm-bootstrap swarm-status swarm-upgrade swarm-rollback swarm-verify stateful

help: ## Show this help
	@awk 'BEGIN {FS = ":.*## "; printf "QuantRule make targets\n\n"} /^[a-zA-Z0-9_-]+:.*## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@printf '\nWindows: run wsl bash tools/secrets.sh and wsl bash tools/pki.sh, then docker compose --env-file secrets\\compose.env up -d.\n'
	@printf 'Fedora: make up COMPOSE="podman compose".\n'

secrets: ## Create the local secret files (idempotent; FORCE=1 rotates)
	@./tools/secrets.sh --out "$(abspath $(SECRETS_DIR))" $(if $(FORCE),--force,)

pki: ## Create the local internal PKI (idempotent; FORCE=1 regenerates)
	@QR_PKI_DIR="$(abspath $(PKI_DIR))" ./tools/pki.sh --out "$(abspath $(PKI_DIR))" $(if $(FORCE),--force,)

up: secrets pki ## Build and start the default stack, wait for health, print the URL
	@command -v curl >/dev/null || { echo "up: curl is required to check /health" >&2; exit 1; }
	$(COMPOSE_ENV) build
	$(COMPOSE_ENV) create gateway
	@digest="$$($(COMPOSE_ENV) images -q gateway | head -n1)"; \
	if [ -z "$$digest" ]; then \
		echo "up: the built gateway image has no id; cannot record the run identity" >&2; \
		exit 1; \
	fi; \
	QR_IMAGE_DIGEST="$$digest" $(COMPOSE_ENV) up -d --wait
	@curl -fsS "$(QR_URL)/health" >/dev/null
	@version="$$(curl -fsS "$(QR_URL)/version" | sed -n 's/.*"product":"\([^"]*\)".*/\1/p')"; \
	if [ -z "$$version" ]; then \
		echo "up: the gateway did not report a product version" >&2; \
		exit 1; \
	fi; \
	printf 'QuantRule v%s ready → %s\n' "$$version" "$(QR_URL)"

compose-check: ## Validate the local and remote Compose files with disposable secrets and PKI
	@tmp="$$(mktemp -d)"; trap 'rm -rf "$$tmp"' EXIT; \
	./tools/secrets.sh --out "$$tmp/secrets" >/dev/null; \
	./tools/pki.sh --out "$$tmp/pki" >/dev/null; \
	QR_SECRETS_DIR="$$tmp/secrets" QR_PKI_DIR="$$tmp/pki" $(COMPOSE) --env-file "$$tmp/secrets/compose.env" -f compose.yaml -f compose.remote.yaml config -q; \
	echo "compose config ok"

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
		echo "scale: model/backtest are not in the active Compose services; nothing was scaled." >&2; \
		echo "scale: this target scales model and backtest to N when the Compose profile exposes them." >&2; \
		exit 1; \
	fi

gate: ## Run the full workspace gate
	./tools/gate.sh

verify: ## Run two equivalent submissions and print the digest
	./tools/verify.sh --config $(CONFIG) --replica $(REPLICA) $(if $(EXPECTED),--expected '$(EXPECTED)')

test: test-fast ## Run the fast unit and integration tier
test-fast: ## Run the fast unit and integration tier
	./tools/test_tiers.sh fast
test-composed: ## Run the composed tier against the real stack
	./tools/test_tiers.sh composed
test-parity: ## Run the port-parity tier
	./tools/test_tiers.sh parity
test-ha: ## Run the opt-in Swarm HA lab (pass ARGS=--plan or ARGS=--confirm)
	./tools/test_tiers.sh ha $(ARGS)

stateful: ## Operate the v3 stateful tier (ARGS="--plan" or network/copy/start/wait/status)
	./tools/stateful.sh $(ARGS)
swarm-validate: ## Validate the v3 HA descriptor and local files
	./tools/swarm.sh validate
swarm-render: ## Render the v3 HA stack, infra skeletons, jobs, and bootstrap
	./tools/swarm.sh render
swarm-bootstrap: ## Create objects, label nodes, migrate, and deploy on a manager
	./tools/swarm.sh bootstrap
swarm-status: ## Show the stack and the nodes labelled for this deployment
	./tools/swarm.sh status
swarm-upgrade: ## Snapshot, render, and roll the stack forward
	./tools/swarm.sh upgrade
swarm-rollback: ## Redeploy the last snapshot; never removes the stack or data
	./tools/swarm.sh rollback
swarm-verify: ## Check the ingress, image digest, and local files
	./tools/swarm.sh verify

remote: ## Start with only :8080 published and a bearer token
	QR_SECRETS_DIR="$(abspath $(SECRETS_DIR))" COMPOSE="$(COMPOSE)" ./tools/remote.sh

token: ## Print a new bearer token
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
