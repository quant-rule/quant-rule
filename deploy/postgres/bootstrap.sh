#!/bin/sh
# Idempotent Postgres bootstrap for the four QuantRule logical databases.
#
# One cluster holds qr_gateway, qr_catalog, qr_evidence, and qr_simulation. Each
# service owns one database and connects with two roles: <database>_owner runs
# its migrations and owns every object, <database>_app serves requests with DML
# only. The script revokes PUBLIC connect and the app role's DDL, so a service
# cannot reach another service's database or create an object in its own.
#
# Run it as a Postgres superuser against the maintenance database. The local
# Compose stack runs it as a one-shot `db-bootstrap` service after Postgres is
# healthy, and the multi-host infrastructure role runs the same service. Both
# paths are idempotent, so an existing volume is upgraded in place and a re-run
# rotates the role passwords to the current environment.
#
# Environment:
#   PGHOST, PGPORT, PGUSER, PGPASSWORD, PGDATABASE
#       The superuser connection. PGUSER defaults to postgres and PGDATABASE to
#       postgres when the caller does not supply them.
#   QR_DB_ROLE_PASSWORD
#       One password for every owner and app role. QR_POSTGRES_PASSWORD is the
#       fallback the Compose stack already carries.
#   QR_<DATABASE>_OWNER_PASSWORD, QR_<DATABASE>_APP_PASSWORD
#       Optional per-role overrides. The v2 descriptor has one Postgres secret,
#       so the multi-host renderer leaves these unset; v3 may inject them.
set -eu

: "${PGHOST:=postgres}"
: "${PGPORT:=5432}"
: "${PGUSER:=postgres}"
: "${PGDATABASE:=postgres}"
export PGHOST PGPORT PGUSER PGDATABASE

role_password=${QR_DB_ROLE_PASSWORD:-${QR_POSTGRES_PASSWORD:-}}
if [ -z "$role_password" ]; then
  echo "error: QR_DB_ROLE_PASSWORD or QR_POSTGRES_PASSWORD must be set" >&2
  exit 1
fi

gateway_owner_password=${QR_GATEWAY_OWNER_PASSWORD:-$role_password}
gateway_app_password=${QR_GATEWAY_APP_PASSWORD:-$role_password}
catalog_owner_password=${QR_CATALOG_OWNER_PASSWORD:-$role_password}
catalog_app_password=${QR_CATALOG_APP_PASSWORD:-$role_password}
evidence_owner_password=${QR_EVIDENCE_OWNER_PASSWORD:-$role_password}
evidence_app_password=${QR_EVIDENCE_APP_PASSWORD:-$role_password}
simulation_owner_password=${QR_SIMULATION_OWNER_PASSWORD:-$role_password}
simulation_app_password=${QR_SIMULATION_APP_PASSWORD:-$role_password}

# The role and database names are fixed. Identifiers in the SQL below are
# therefore literals, and a password is the only value interpolated.
psql -q -v ON_ERROR_STOP=1 \
  -v gateway_owner_password="$gateway_owner_password" \
  -v gateway_app_password="$gateway_app_password" \
  -v catalog_owner_password="$catalog_owner_password" \
  -v catalog_app_password="$catalog_app_password" \
  -v evidence_owner_password="$evidence_owner_password" \
  -v evidence_app_password="$evidence_app_password" \
  -v simulation_owner_password="$simulation_owner_password" \
  -v simulation_app_password="$simulation_app_password" <<'SQL'
-- Roles. CREATE ROLE has no IF NOT EXISTS, so the catalogue check decides
-- whether the statement is emitted. ALTER ROLE always runs and keeps a re-run
-- aligned with the current secret.
SELECT format('CREATE ROLE %I LOGIN', 'qr_gateway_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_gateway_owner')
\gexec
ALTER ROLE qr_gateway_owner WITH LOGIN PASSWORD :'gateway_owner_password';
SELECT format('CREATE ROLE %I LOGIN', 'qr_gateway_app')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_gateway_app')
\gexec
ALTER ROLE qr_gateway_app WITH LOGIN PASSWORD :'gateway_app_password';
SELECT format('CREATE ROLE %I LOGIN', 'qr_catalog_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_catalog_owner')
\gexec
ALTER ROLE qr_catalog_owner WITH LOGIN PASSWORD :'catalog_owner_password';
SELECT format('CREATE ROLE %I LOGIN', 'qr_catalog_app')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_catalog_app')
\gexec
ALTER ROLE qr_catalog_app WITH LOGIN PASSWORD :'catalog_app_password';
SELECT format('CREATE ROLE %I LOGIN', 'qr_evidence_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_evidence_owner')
\gexec
ALTER ROLE qr_evidence_owner WITH LOGIN PASSWORD :'evidence_owner_password';
SELECT format('CREATE ROLE %I LOGIN', 'qr_evidence_app')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_evidence_app')
\gexec
ALTER ROLE qr_evidence_app WITH LOGIN PASSWORD :'evidence_app_password';
SELECT format('CREATE ROLE %I LOGIN', 'qr_simulation_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_simulation_owner')
\gexec
ALTER ROLE qr_simulation_owner WITH LOGIN PASSWORD :'simulation_owner_password';
SELECT format('CREATE ROLE %I LOGIN', 'qr_simulation_app')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qr_simulation_app')
\gexec
ALTER ROLE qr_simulation_app WITH LOGIN PASSWORD :'simulation_app_password';

-- Databases. CREATE DATABASE cannot run inside a transaction block, so each
-- emitted statement runs on its own; ALTER DATABASE converges an older owner.
SELECT format('CREATE DATABASE %I OWNER %I', 'qr_gateway', 'qr_gateway_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'qr_gateway')
\gexec
ALTER DATABASE qr_gateway OWNER TO qr_gateway_owner;
SELECT format('CREATE DATABASE %I OWNER %I', 'qr_catalog', 'qr_catalog_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'qr_catalog')
\gexec
ALTER DATABASE qr_catalog OWNER TO qr_catalog_owner;
SELECT format('CREATE DATABASE %I OWNER %I', 'qr_evidence', 'qr_evidence_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'qr_evidence')
\gexec
ALTER DATABASE qr_evidence OWNER TO qr_evidence_owner;
SELECT format('CREATE DATABASE %I OWNER %I', 'qr_simulation', 'qr_simulation_owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'qr_simulation')
\gexec
ALTER DATABASE qr_simulation OWNER TO qr_simulation_owner;

-- qr_gateway: the gateway schema.
\connect qr_gateway
REVOKE ALL ON DATABASE qr_gateway FROM PUBLIC;
REVOKE TEMP ON DATABASE qr_gateway FROM PUBLIC;
GRANT CONNECT, CREATE ON DATABASE qr_gateway TO qr_gateway_owner;
GRANT CONNECT ON DATABASE qr_gateway TO qr_gateway_app;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA public TO qr_gateway_owner;
GRANT USAGE ON SCHEMA public TO qr_gateway_app;
CREATE SCHEMA IF NOT EXISTS gateway AUTHORIZATION qr_gateway_owner;
ALTER SCHEMA gateway OWNER TO qr_gateway_owner;
REVOKE ALL ON SCHEMA gateway FROM PUBLIC;
GRANT USAGE ON SCHEMA gateway TO qr_gateway_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA gateway TO qr_gateway_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA gateway TO qr_gateway_app;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA gateway TO qr_gateway_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_gateway_owner IN SCHEMA gateway
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO qr_gateway_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_gateway_owner IN SCHEMA gateway
  GRANT USAGE, SELECT ON SEQUENCES TO qr_gateway_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_gateway_owner IN SCHEMA gateway
  GRANT EXECUTE ON FUNCTIONS TO qr_gateway_app;

-- qr_catalog: the catalog schema.
\connect qr_catalog
REVOKE ALL ON DATABASE qr_catalog FROM PUBLIC;
REVOKE TEMP ON DATABASE qr_catalog FROM PUBLIC;
GRANT CONNECT, CREATE ON DATABASE qr_catalog TO qr_catalog_owner;
GRANT CONNECT ON DATABASE qr_catalog TO qr_catalog_app;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA public TO qr_catalog_owner;
GRANT USAGE ON SCHEMA public TO qr_catalog_app;
CREATE SCHEMA IF NOT EXISTS catalog AUTHORIZATION qr_catalog_owner;
ALTER SCHEMA catalog OWNER TO qr_catalog_owner;
REVOKE ALL ON SCHEMA catalog FROM PUBLIC;
GRANT USAGE ON SCHEMA catalog TO qr_catalog_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA catalog TO qr_catalog_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA catalog TO qr_catalog_app;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA catalog TO qr_catalog_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_catalog_owner IN SCHEMA catalog
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO qr_catalog_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_catalog_owner IN SCHEMA catalog
  GRANT USAGE, SELECT ON SEQUENCES TO qr_catalog_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_catalog_owner IN SCHEMA catalog
  GRANT EXECUTE ON FUNCTIONS TO qr_catalog_app;

-- qr_evidence: runs, research, and stage.
\connect qr_evidence
REVOKE ALL ON DATABASE qr_evidence FROM PUBLIC;
REVOKE TEMP ON DATABASE qr_evidence FROM PUBLIC;
GRANT CONNECT, CREATE ON DATABASE qr_evidence TO qr_evidence_owner;
GRANT CONNECT ON DATABASE qr_evidence TO qr_evidence_app;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA public TO qr_evidence_owner;
GRANT USAGE ON SCHEMA public TO qr_evidence_app;
CREATE SCHEMA IF NOT EXISTS runs AUTHORIZATION qr_evidence_owner;
ALTER SCHEMA runs OWNER TO qr_evidence_owner;
REVOKE ALL ON SCHEMA runs FROM PUBLIC;
GRANT USAGE ON SCHEMA runs TO qr_evidence_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA runs TO qr_evidence_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA runs TO qr_evidence_app;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA runs TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA runs
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA runs
  GRANT USAGE, SELECT ON SEQUENCES TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA runs
  GRANT EXECUTE ON FUNCTIONS TO qr_evidence_app;
CREATE SCHEMA IF NOT EXISTS research AUTHORIZATION qr_evidence_owner;
ALTER SCHEMA research OWNER TO qr_evidence_owner;
REVOKE ALL ON SCHEMA research FROM PUBLIC;
GRANT USAGE ON SCHEMA research TO qr_evidence_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA research TO qr_evidence_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA research TO qr_evidence_app;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA research TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA research
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA research
  GRANT USAGE, SELECT ON SEQUENCES TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA research
  GRANT EXECUTE ON FUNCTIONS TO qr_evidence_app;
CREATE SCHEMA IF NOT EXISTS stage AUTHORIZATION qr_evidence_owner;
ALTER SCHEMA stage OWNER TO qr_evidence_owner;
REVOKE ALL ON SCHEMA stage FROM PUBLIC;
GRANT USAGE ON SCHEMA stage TO qr_evidence_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA stage TO qr_evidence_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA stage TO qr_evidence_app;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA stage TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA stage
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA stage
  GRANT USAGE, SELECT ON SEQUENCES TO qr_evidence_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_evidence_owner IN SCHEMA stage
  GRANT EXECUTE ON FUNCTIONS TO qr_evidence_app;

-- qr_simulation: the simulation schema.
\connect qr_simulation
REVOKE ALL ON DATABASE qr_simulation FROM PUBLIC;
REVOKE TEMP ON DATABASE qr_simulation FROM PUBLIC;
GRANT CONNECT, CREATE ON DATABASE qr_simulation TO qr_simulation_owner;
GRANT CONNECT ON DATABASE qr_simulation TO qr_simulation_app;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA public TO qr_simulation_owner;
GRANT USAGE ON SCHEMA public TO qr_simulation_app;
CREATE SCHEMA IF NOT EXISTS simulation AUTHORIZATION qr_simulation_owner;
ALTER SCHEMA simulation OWNER TO qr_simulation_owner;
REVOKE ALL ON SCHEMA simulation FROM PUBLIC;
GRANT USAGE ON SCHEMA simulation TO qr_simulation_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA simulation TO qr_simulation_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA simulation TO qr_simulation_app;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA simulation TO qr_simulation_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_simulation_owner IN SCHEMA simulation
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO qr_simulation_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_simulation_owner IN SCHEMA simulation
  GRANT USAGE, SELECT ON SEQUENCES TO qr_simulation_app;
ALTER DEFAULT PRIVILEGES FOR ROLE qr_simulation_owner IN SCHEMA simulation
  GRANT EXECUTE ON FUNCTIONS TO qr_simulation_app;
SQL

echo "QuantRule databases ready: qr_gateway, qr_catalog, qr_evidence, qr_simulation"
