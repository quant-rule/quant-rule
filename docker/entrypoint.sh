#!/bin/sh
# Select the service binary for the one QuantRule image.
#
# Compose names the binary as the container command. With no command the image
# starts the gateway, which is the only service that publishes a port. The
# `--version` branch is the image identity probe: it prints the workspace version
# baked at build time and never starts a service.
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

exec "$@"
