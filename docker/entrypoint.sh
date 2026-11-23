#!/bin/sh
# Select the service binary for the one QuantRule image.
#
# Compose names the binary as the container command. With no command the image
# starts the gateway, which is the only service with a public HTTP surface. The
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

fail() {
    printf 'quantrule-entry: %s\n' "$*" >&2
    exit 1
}

# A Swarm task has a private overlay address that is not known when the
# descriptor is rendered. The runtime still requires a private bind when
# QR_INTERNAL_ONLY=1, so replace an unspecified host with the task address and
# keep the port. This is the Swarm/PLAT-001 boundary; a failed resolution is
# fatal rather than a public fallback.
resolve_private_bind() {
    value=$1
    case "$value" in
        0.0.0.0:*)
            command -v getent >/dev/null 2>&1 || fail "getent is required to resolve the task address"
            hostname_value=$(uname -n 2>/dev/null || true)
            [ -n "$hostname_value" ] || fail "cannot read the task hostname"
            # Read the resolver output line by line instead of adding awk to
            # the minimal runtime image. A loopback-only mapping is not a
            # reachable overlay address, so it fails closed.
            address=""
            while read -r candidate _rest; do
                [ -n "$candidate" ] || continue
                case "$candidate" in
                    127.*|::1) continue ;;
                esac
                address=$candidate
                break
            done <<EOF
$(getent hosts "$hostname_value" 2>/dev/null || true)
EOF
            [ -n "$address" ] || fail "cannot resolve a non-loopback address for $hostname_value"
            port=${value#*:}
            case "$address" in
                *:*) address="[$address]" ;;
            esac
            printf '%s:%s' "$address" "$port"
            ;;
        *)
            printf '%s' "$value"
            ;;
    esac
}

case "${1:-}" in
    --version|-version)
        printf 'quantrule-server %s\n' "$(cat /etc/quantrule-version)"
        exit 0
        ;;
esac

if [ "${QR_INTERNAL_ONLY:-0}" = "1" ]; then
    if [ -n "${QR_BIND:-}" ]; then
        resolved=$(resolve_private_bind "$QR_BIND") || exit 1
        QR_BIND=$resolved
        export QR_BIND
    fi
    if [ -n "${QR_METRICS_BIND:-}" ]; then
        resolved=$(resolve_private_bind "$QR_METRICS_BIND") || exit 1
        QR_METRICS_BIND=$resolved
        export QR_METRICS_BIND
    fi
fi

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
