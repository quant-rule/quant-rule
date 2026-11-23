#!/usr/bin/env bash
# Create the local internal PKI for PostgreSQL, RabbitMQ and MinIO.
#
# The CA model is offline root -> online intermediate -> 180-day leaf. The root
# key is generated here and never leaves the operator host; only `ca.crt`
# (intermediate + root) is mounted into a service. The script is idempotent and
# uses openssl only. No private key is tracked by Git.
#
# Usage:
#   tools/pki.sh                 create missing certificates under ./pki
#   tools/pki.sh --out DIR       write to DIR instead
#   tools/pki.sh --force         regenerate the whole directory (explicit)
#
# Environment:
#   PKI_EXTRA_SANS   comma-separated DNS names or IP addresses added to every
#                    server certificate, for example "10.10.0.20,quantrule.dev"
#
# The directory is mode 700, private keys are mode 600 and certificates are
# mode 644. `make up` runs this before the stack starts; the Compose PKI init
# copies the service-owned files into a volume with the database/broker/store
# UID and keeps the key at 0600.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
out="${QR_PKI_DIR:-$root/pki}"
force=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      [ "$#" -ge 2 ] || { echo "pki: --out needs a directory" >&2; exit 2; }
      out="$2"
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    -h | --help)
      sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "pki: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

command -v openssl >/dev/null 2>&1 || { echo "pki: openssl is required" >&2; exit 1; }

required="$out/ca.crt $out/postgres/server.crt $out/rabbitmq/server.crt $out/minio/public.crt $out/minio/CAs/ca.crt"
if [ "$force" -eq 0 ]; then
  missing=0
  for path in $required; do
    [ -f "$path" ] || missing=1
  done
  if [ "$missing" -eq 0 ]; then
    echo "pki: ready in $out (root 10y, intermediate 5y, leaves 180d)"
    exit 0
  fi
else
  rm -rf "$out"
fi

umask 077
mkdir -p "$out/ca" "$out/postgres" "$out/rabbitmq" "$out/minio/CAs"
chmod 700 "$out" "$out/ca" "$out/postgres" "$out/rabbitmq" "$out/minio" "$out/minio/CAs"

work="$(mktemp -d "${TMPDIR:-/tmp}/quantrule-pki.XXXXXX")"
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

# Offline root: ten years, CA only, never mounted.
openssl genrsa -out "$out/ca/root.key" 4096 >/dev/null 2>&1
chmod 600 "$out/ca/root.key"
openssl req -x509 -new -sha256 -key "$out/ca/root.key" -days 3650 \
  -subj "/CN=QuantRule Offline Root CA" \
  -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign" \
  -out "$out/ca/root.crt"
chmod 644 "$out/ca/root.crt"

# Online intermediate: five years, signed by the root.
openssl genrsa -out "$out/ca/intermediate.key" 4096 >/dev/null 2>&1
chmod 600 "$out/ca/intermediate.key"
openssl req -new -sha256 -key "$out/ca/intermediate.key" \
  -subj "/CN=QuantRule Online Intermediate CA" -out "$work/intermediate.csr"
cat > "$work/intermediate.ext" <<'EOF'
basicConstraints=critical,CA:TRUE,pathlen:0
keyUsage=critical,keyCertSign,cRLSign
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid,issuer
EOF
openssl x509 -req -sha256 -in "$work/intermediate.csr" \
  -CA "$out/ca/root.crt" -CAkey "$out/ca/root.key" -CAcreateserial \
  -days 1825 -extfile "$work/intermediate.ext" \
  -out "$out/ca/intermediate.crt" >/dev/null 2>&1
chmod 644 "$out/ca/intermediate.crt"
cat "$out/ca/intermediate.crt" "$out/ca/root.crt" > "$out/ca.crt"
chmod 644 "$out/ca.crt"

extra_sans="${PKI_EXTRA_SANS:-}"
leaf() {
  local name="$1" key="$2" cert="$3"
  openssl genrsa -out "$key" 2048 >/dev/null 2>&1
  chmod 600 "$key"
  openssl req -new -sha256 -key "$key" -subj "/CN=$name" -out "$work/$name.csr"
  {
    printf 'basicConstraints=critical,CA:FALSE\n'
    printf 'keyUsage=critical,digitalSignature,keyEncipherment\n'
    printf 'extendedKeyUsage=serverAuth\n'
    printf 'subjectAltName=@alt\n\n[alt]\n'
    printf 'DNS.1=%s\nDNS.2=localhost\nIP.1=127.0.0.1\n' "$name"
    local index=3 ip_index=2
    if [ -n "$extra_sans" ]; then
      local old_ifs="$IFS"
      IFS=','
      for san in $extra_sans; do
        san="$(printf '%s' "$san" | tr -d '[:space:]')"
        [ -n "$san" ] || continue
        case "$san" in
          *[!0-9.]*)
            printf 'DNS.%s=%s\n' "$index" "$san"
            index=$((index + 1))
            ;;
          *)
            printf 'IP.%s=%s\n' "$ip_index" "$san"
            ip_index=$((ip_index + 1))
            ;;
        esac
      done
      IFS="$old_ifs"
    fi
  } > "$work/$name.ext"
  openssl x509 -req -sha256 -in "$work/$name.csr" \
    -CA "$out/ca/intermediate.crt" -CAkey "$out/ca/intermediate.key" -CAcreateserial \
    -days 180 -extfile "$work/$name.ext" -out "$cert" >/dev/null 2>&1
  chmod 644 "$cert"
}

leaf postgres "$out/postgres/server.key" "$out/postgres/server.crt"
cp "$out/ca.crt" "$out/postgres/ca.crt"
chmod 644 "$out/postgres/ca.crt"

leaf rabbitmq "$out/rabbitmq/server.key" "$out/rabbitmq/server.crt"
cp "$out/ca.crt" "$out/rabbitmq/ca.crt"
chmod 644 "$out/rabbitmq/ca.crt"

leaf minio "$out/minio/private.key" "$out/minio/public.crt"
cp "$out/ca.crt" "$out/minio/CAs/ca.crt"
chmod 644 "$out/minio/CAs/ca.crt"

for dir in "$out/postgres" "$out/rabbitmq" "$out/minio" "$out/minio/CAs"; do
  chmod 700 "$dir"
done
rm -f "$out/ca/root.srl" "$out/ca/intermediate.srl"

echo "pki: ready in $out (root 10y, intermediate 5y, leaves 180d)"
echo "pki: mount ca.crt only; the root and intermediate keys stay on this host"
