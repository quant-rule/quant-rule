# The derived Patroni image for HA-013.
#
# The pinned upstream image does not contain pgBackRest. This image adds the
# distribution package so `archive-push`, `archive-get`, backup, verify, info,
# and restore run inside the same Patroni container as PostgreSQL. The base is
# the exact registry digest, not a tag; `build.sh` prints the built digest for
# the descriptor and refuses a changed base.

FROM batonogov/patroni-docker@sha256:0de7dc8bc5919c76721bee1475ed30155b1848c0a6bac7298605bf39eef1a7eb

USER root

LABEL org.opencontainers.image.title="quantrule-patroni-pgbackrest" \
      org.opencontainers.image.description="Patroni and PostgreSQL with the pgBackRest archive tools for QuantRule HA-013" \
      org.opencontainers.image.source="https://github.com/quant-rule/quant-rule" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.base.name="docker.io/batonogov/patroni-docker" \
      org.opencontainers.image.base.digest="sha256:0de7dc8bc5919c76721bee1475ed30155b1848c0a6bac7298605bf39eef1a7eb"

RUN apt-get update \
    && apt-get install -y --no-install-recommends pgbackrest \
    && rm -rf /var/lib/apt/lists/* \
    && pgbackrest version

# The upstream entrypoint and default user are preserved. The unit supplies
# `PATRONI_*` and `PGBACKREST_*` values from mode-600 files at runtime; none is
# baked into this image.
