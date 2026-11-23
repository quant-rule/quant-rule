# Fixture Ingress Certificate

`ingress.crt` and `ingress-key.pem` are fake cert-mode fixtures. They are not a
real certificate or private key, are not used to serve TLS, and must never be
deployed. The test suite copies them to a temporary directory and sets the key
copy to mode 600.

The `.pem` key name and the absent PEM private-key block keep the repository
private-key checker honest. The deployment uses
`/etc/quantrule/pki/ingress.crt` and `/etc/quantrule/pki/ingress-key.pem`; the
renderer writes `pki/ingress.crt` and `pki/ingress.key`. The contract is
[`docs/config/2026-11-23/ingress.md`](../../../docs/config/2026-11-23/ingress.md).
