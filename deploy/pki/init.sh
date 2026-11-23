#!/bin/sh
# Copy the mounted server certificates into a service-owned volume.
#
# The host PKI is mode 700 with mode-600 keys. PostgreSQL, RabbitMQ and MinIO
# each run as a non-root UID and cannot read those host files directly. Running
# the database as root or chmod-ing a private key to 0644 would weaken the
# deployment, so this one-shot root container copies each file into a named
# volume, gives it the service UID and keeps the key at 0600. The long-running
# container still starts as the stock non-root user.
#
# It runs on every `make up`; the copy is small and makes a rotated certificate
# visible without touching the host directory.
set -eu

postgres_uid="${QR_PKI_POSTGRES_UID:-999}"
postgres_gid="${QR_PKI_POSTGRES_GID:-999}"
rabbitmq_uid="${QR_PKI_RABBITMQ_UID:-999}"
rabbitmq_gid="${QR_PKI_RABBITMQ_GID:-999}"
minio_uid="${QR_PKI_MINIO_UID:-1001}"
minio_gid="${QR_PKI_MINIO_GID:-1001}"

copy_server() {
  source="$1"
  destination="$2"
  uid="$3"
  gid="$4"
  key="$5"
  cert="$6"
  ca="$7"

  install -d -m 700 -o "$uid" -g "$gid" "$destination"
  for name in "$key" "$cert" "$ca"; do
    if [ ! -f "$source/$name" ]; then
      echo "pki-init: $source/$name is missing; run 'make pki'" >&2
      exit 1
    fi
  done
  install -m 600 -o "$uid" -g "$gid" "$source/$key" "$destination/$key"
  install -m 644 -o "$uid" -g "$gid" "$source/$cert" "$destination/$cert"
  install -m 644 -o "$uid" -g "$gid" "$source/$ca" "$destination/$ca"
}

copy_server /src/postgres /pki/postgres "$postgres_uid" "$postgres_gid" \
  server.key server.crt ca.crt
copy_server /src/rabbitmq /pki/rabbitmq "$rabbitmq_uid" "$rabbitmq_gid" \
  server.key server.crt ca.crt

install -d -m 700 -o "$minio_uid" -g "$minio_gid" /pki/minio /pki/minio/CAs
for name in private.key public.crt CAs/ca.crt; do
  if [ ! -f "/src/minio/$name" ]; then
    echo "pki-init: /src/minio/$name is missing; run 'make pki'" >&2
    exit 1
  fi
done
install -m 600 -o "$minio_uid" -g "$minio_gid" /src/minio/private.key /pki/minio/private.key
install -m 644 -o "$minio_uid" -g "$minio_gid" /src/minio/public.crt /pki/minio/public.crt
install -m 644 -o "$minio_uid" -g "$minio_gid" /src/minio/CAs/ca.crt /pki/minio/CAs/ca.crt

echo "pki-init: certificates staged for postgres, rabbitmq and minio"
