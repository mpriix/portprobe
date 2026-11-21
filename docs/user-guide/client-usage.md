# Client Probe Mode (`--health-check`)

**Document Reference:** `PORTPROBE-DOC-UG-03`  
**Document Version:** `2.0.0`  
**Classification:** Comprehensive User Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Client Health Check Overview

PortProbe is not only a daemon; it also contains a built-in active client probe engine invoked via the `--health-check` flag. This eliminates the need to install third-party testing tools (like `nc`, `curl`, or Python) on client machines or inside minimal CI/CD runner environments.

### Key Characteristics:
- **Strict Verification**: Asserts that the remote endpoint not only establishes a TCP connection, but also returns the authoritative PortProbe verification banner (`PORTPROBE: TCP CONNECTION SUCCESSFUL\n`).
- **Eliminates False Positives**: Rejects non-PortProbe listeners, closed ports, rate-limited responses, and CIDR forbidden banners.
- **Standard Process Exit Codes**: Exits `0` on verified reachability; exits `1` on failure, timeout, or rejection.
- **TLS & mTLS Ready**: Capable of verifying encrypted endpoints, enforcing SNI domain validation, and trusting custom private PKI CA roots.

---

## 2. Basic Client Health Check

To test reachability to a remote server `10.0.0.50` on port `8080`:

```bash
portprobe --health-check --bind 10.0.0.50 --port 8080
```

> [!NOTE]  
> When `--health-check` is passed, `--bind` represents the **target remote host IP** to probe, and `--port` represents the target port.

### Success Output:
```text
PortProbe healthcheck SUCCESS: 10.0.0.50:8080 is reachable
```
*(Exit code: 0)*

### Failure Output (Connection Refused):
```text
PortProbe healthcheck FAILURE: Connection refused (os error 111)
```
*(Exit code: 1)*

### Failure Output (CIDR Rejected):
If the client IP is blocked by the daemon's CIDR filter and `silent_reject = false`:
```text
PortProbe healthcheck FAILURE: Healthcheck rejected: received non-success banner: "PORTPROBE: CONNECTION NOT ALLOWED (CIDR FILTER)"
```
*(Exit code: 1)*

---

## 3. Encrypted TLS Client Probing

When the remote PortProbe daemon has TLS encryption enabled, run the client probe with TLS flags:

```bash
portprobe \
  --health-check \
  --bind 10.0.0.50 \
  --port 443 \
  --health-check-tls \
  --health-check-domain "probe.example.com" \
  --health-check-ca "/etc/ssl/certs/corporate-ca.pem"
```

### Argument Details:
- `--health-check-tls`: Establishes an encrypted TLS session post-TCP handshake.
- `--health-check-domain <DOMAIN>`: Sets the Server Name Indication (SNI) hostname and verifies that the certificate presented by the daemon matches this domain. Defaults to `localhost` if omitted.
- `--health-check-ca <PATH>`: Path to custom CA certificate PEM bundle to trust for certificate validation.

### TLS Verification Failure Example (Invalid Domain / Hostname Mismatch):
```text
PortProbe healthcheck FAILURE: invalid peer certificate: CertNotValidForName
```
*(Exit code: 1)*

---

## 4. Integrating Client Probe into Scripts & CI/CD

Because `--health-check` produces deterministic exit codes, it integrates seamlessly into shell scripts and deployment automation:

```bash
#!/usr/bin/env bash
set -e

TARGET_HOST="10.200.1.50"
TARGET_PORT="8080"

echo "Validating network reachability to ${TARGET_HOST}:${TARGET_PORT}..."

if portprobe --health-check --bind "$TARGET_HOST" --port "$TARGET_PORT"; then
    echo "Network reachability confirmed. Proceeding with deployment."
else
    echo "ERROR: Network connectivity failed. Aborting pipeline." >&2
    exit 1
fi
```
