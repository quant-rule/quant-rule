#!/bin/sh
# Select the service binary for the one QuantRule image.
#
# Compose names the binary as the container command. With no command the image
# starts the gateway, which is the only service that publishes a port. The
# `--version` branch is the image identity probe: it prints the workspace version
# baked at build time and never starts a service.
#
# Docker Compose file secrets are bind mounts, so a host file at mode 600 is
# owned by the operator and cannot be read by the image's UID 10001. The
# entrypoint therefore starts as root, copies the mounted files into a private
# mode-700 directory owned by UID 10001, rewrites the `*_FILE` values to that
# copy, and drops privileges before exec. The host files and the mount stay
# mode 600; no secret becomes an environment value.
set -eu

case "${1:-}" in
    --version|-version)
        printf 'quantrule-server %s\n' "$(cat /etc/quantrule-version)"
        exit 0
        ;;
esac

if [ "$#" -eq 0 ]; then
    set -- qr-gateway
fi

if [ "$(id -u)" -eq 0 ] && [ -d /run/secrets ]; then
    stage=/run/quantrule-secrets
    rm -rf "$stage"
    mkdir -p "$stage"
    for source in /run/secrets/*; do
        [ -f "$source" ] || continue
        name=${source##*/}
        cp "$source" "$stage/$name"
        chown 10001:10001 "$stage/$name"
        chmod 400 "$stage/$name"
    done
    chown 10001:10001 "$stage"
    chmod 700 "$stage"

    # Only a runtime path is rewritten; values are never read or printed.
    for name in $(env | sed -n 's/^\(QR_[A-Z0-9_]*_FILE\)=.*/\1/p'); do
        eval "value=\"\${$name}\""
        case "$value" in
            /run/secrets/*)
                export "$name=$stage/${value#/run/secrets/}"
                ;;
        esac
    done
fi

if [ "$(id -u)" -eq 0 ]; then
    exec setpriv --reuid=10001 --regid=10001 --clear-groups "$@"
fi

exec "$@"
