# Fixture Ingress Certificate and Stateful PKI Layout

`ingress.crt` and `ingress-key.pem` are fake cert-mode fixtures. They are not a
real certificate or private key, are not used to serve TLS, and must never be
deployed. The test suite copies them to a temporary directory and sets the key
copy to mode 600.

The `.pem` key name and the absent PEM private-key block keep the repository
private-key checker honest. The deployment uses
`/etc/quantrule/pki/ingress.crt` and `/etc/quantrule/pki/ingress-key.pem`; the
renderer writes `pki/ingress.crt` and `pki/ingress.key`. The contract is
[`docs/config/2026-11-23/ingress.md`](../../../docs/config/2026-11-23/ingress.md).

## Stateful PKI

`tools/pki.sh` writes the stateful leaves under the same internal CA:

```text
pki/
  ca.crt                     intermediate + root bundle (644)
  postgres/server.crt .key   PostgreSQL server leaf; DNS SAN `postgres`
  etcd/server.crt .key       etcd client-listener leaf
  etcd/peer.crt .key         etcd peer leaf (serverAuth + clientAuth)
  etcd/client.crt .key       Patroni etcd3 client leaf (clientAuth)
  rabbitmq/server.crt .key   RabbitMQ server leaf; DNS SAN `rabbitmq`
```

The renderer reads the Postgres and etcd leaves from the parent directory of
`endpoints.database.tls_ca` and the RabbitMQ leaf from the parent of
`endpoints.queue.tls_ca`, copies them to `out/pki/` (the RabbitMQ leaf under
`pki/rabbitmq/`), and lists them in `stateful-manifest.json` with a per-entry
`hosts` selector. Private keys must be mode 600; certificates are 644. The same
Postgres leaf is copied to every Patroni node, so its SAN must include
`postgres` plus every `patroni-<host>`; the etcd server and peer leaves need every
`etcd-<host>`; the RabbitMQ leaf needs `rabbitmq` plus every `rabbitmq-<host>`.
`qr-deploy check` queries the rendered or source leaves through
`QR_DEPLOY_OPENSSL` and fails the stateful certificate row when a name is
missing. The required sample set is
`PKI_EXTRA_SANS="patroni-infra-1,patroni-infra-2,patroni-infra-3,etcd-infra-1,etcd-infra-2,etcd-infra-3,rabbitmq-infra-1,rabbitmq-infra-2,rabbitmq-infra-3"`;
the lab uses `patroni-lab-1..3,etcd-lab-1..3,rabbitmq-lab-1..3`. The tests stage fake `.crt` and
`.key` files in a temporary tree with mode 600; no private key is committed.
Contracts:
[`docs/config/2026-11-23/patroni-cluster.md`](../../../docs/config/2026-11-23/patroni-cluster.md)
and [`docs/config/2026-11-23/rabbitmq-cluster.md`](../../../docs/config/2026-11-23/rabbitmq-cluster.md).
