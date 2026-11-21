# Test Fixture Certificates & Keys

> [!CAUTION]
> **DO NOT USE IN PRODUCTION ENVIRONMENTS**
>
> All X.509 certificates, certificate signing requests, and RSA private keys in this directory are **ephemeral, self-signed test fixtures**. They are exclusively utilized by automated unit and integration tests (`tests/networking_tests.rs`, `tests/security_tests.rs`) to validate TLS 1.3 encryption, certificate expiration parsing, and mutual TLS (mTLS) client verification.
>
> None of these keys represent actual production credentials, secrets, or infrastructure identities.

## Fixture Inventory

- `ca.crt` / `ca.key`: Local test Certificate Authority (CN: `PortProbeTestCA`).
- `server.crt` / `server.key`: Test server TLS certificate signed by `PortProbeTestCA` (CN: `localhost`).
- `client.crt` / `client.key`: Valid client certificate signed by `PortProbeTestCA` (CN: `PortProbeTestClient`) for mTLS tests.
- `untrusted_ca.crt` / `untrusted_ca.key`: Independent untrusted CA for rejection testing.
- `untrusted_client.crt` / `untrusted_client.key`: Client certificate signed by the untrusted CA (must be rejected during mTLS handshakes).
- `expired.crt` / `expired.key`: Self-signed certificate with expired validity window.
- `invalid.pem`: Malformed ASCII PEM content to test certificate parser error handling.
