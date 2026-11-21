# PortProbe CLI Reference Manual

**Document Reference:** `PORTPROBE-DOC-REF-01`  
**Document Version:** `2.0.0`  
**Classification:** Reference Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. CLI Command Overview

PortProbe is invoked as a single unified executable:

```text
portprobe [OPTIONS]
```

It operates in **Daemon Mode** by default, or in **Client Probe Mode** when `--health-check` is passed.

---

## 2. Quick-Reference Option Matrix

| Flag / Option | Short | Environment Variable | Default | Valid Range / Values | Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `--port` | `-p` | `PORTPROBE_PORT` | None | `1`–`65535` | Single TCP port to listen on or probe. |
| `--ports` | | `PORTPROBE_PORTS` | None | Comma-delimited list / ranges | Multiple ports / ranges (e.g., `80,443,8080-8085`, max 50 ports). |
| `--bind` | `-b` | `PORTPROBE_BIND` | `127.0.0.1` | Valid IPv4 or IPv6 address | IP address to bind to (daemon) or target host IP (health check). |
| `--timeout` | `-t` | `PORTPROBE_TIMEOUT` | `15` | `1`–`3600` seconds | Connection lifetime before automatic termination. |
| `--max-connections` | | `PORTPROBE_MAX_CONNECTIONS` | `100` | `1`–`10000` | Maximum concurrent connections across all clients. |
| `--max-connections-per-ip` | | `PORTPROBE_MAX_CONNECTIONS_PER_IP` | `10` | `1`–`1000` | Maximum simultaneous connections from a single IP. |
| `--max-rate` | | `PORTPROBE_MAX_RATE` | `20` | `1`–`1000` req/s | Maximum accepted connections per second globally (token bucket). |
| `--max-rate-per-ip` | | `PORTPROBE_MAX_RATE_PER_IP` | `5` | `1`–`1000` req/s | Maximum accepted connections per second per IP (token bucket). |
| `--allow-cidr` | | None | None | Comma-delimited CIDRs | Ingress subnet allowlist (e.g., `10.0.0.0/8,192.168.1.0/24`). |
| `--silent-reject` | | `PORTPROBE_SILENT_REJECT` | `true` | `true`, `false` | Silently drop rejected connections without diagnostic banners. |
| `--hold-open` | | `PORTPROBE_HOLD_OPEN` | `false` | `true`, `false` | Keep TCP socket open post-response until timeout expires. |
| `--config` | `-c` | `PORTPROBE_CONFIG` | None | Valid file path | Path to TOML configuration file (e.g., `/etc/portprobe/portprobe.toml`). |
| `--log-format` | | `PORTPROBE_LOG_FORMAT` | `text` | `text`, `json` | Log output format for terminal viewing or SIEM ingestion. |
| `--health-check` | | None | `false` | Boolean flag | Execute active client healthcheck against configured bind and port. |
| `--health-check-tls` | | None | `false` | Boolean flag | Connect with TLS encryption during client healthcheck. |
| `--health-check-domain` | | None | `localhost` | Valid hostname / SNI | Expected TLS domain name / SNI for healthcheck certificate verification. |
| `--health-check-ca` | | None | None | Valid file path | Path to custom CA certificate PEM bundle for healthcheck TLS validation. |
| `--admin-port` | | `PORTPROBE_ADMIN_PORT` | None | `1`–`65535` | Localhost port for Prometheus `/metrics` and `/healthz` HTTP server. |
| `--admin-token` | | `PORTPROBE_ADMIN_TOKEN` | None | String | Bearer token for authenticating admin endpoint (*deprecated on CLI*). |
| `--admin-token-file` | | `PORTPROBE_ADMIN_TOKEN_FILE`| None | Valid file path | Path to file containing Bearer token with `0600`/`0400` permissions. |
| `--tls-min-version` | | `PORTPROBE_TLS_MIN_VERSION`| `1.2` | `1.2`, `1.3` | Minimum TLS version (`1.3` enforces strict TLS 1.3 only). |
| `--tls-cert` | | `PORTPROBE_TLS_CERT` | None | Valid file path | Path to TLS server certificate chain PEM file. |
| `--tls-key` | | `PORTPROBE_TLS_KEY` | None | Valid file path | Path to TLS server private key PEM file (enforces `0600`/`0400`). |
| `--tls-client-ca` | | `PORTPROBE_TLS_CLIENT_CA` | None | Valid file path | Path to CA bundle for mutual TLS (mTLS) client verification. |
| `--user` | | `PORTPROBE_USER` | None | Valid POSIX user | Unprivileged user account to drop privileges to post-bind. |
| `--group` | | `PORTPROBE_GROUP` | None | Valid POSIX group | Unprivileged group account to drop privileges to post-bind. |
| `--service` | | None | `false` | Boolean flag | Run as background Windows Service (*Operational Workaround: NSSM*). |
| `--help` | `-h` | None | None | None | Print help information. |
| `--version` | `-V` | None | None | None | Print version information. |

---

## 3. Detailed Parameter Specifications

### `-p, --port <PORT>`
- **Description**: Specifies the TCP port to listen on in Daemon mode, or the target port to probe in Client mode.
- **Accepted Values**: Integer `1` to `65535`.
- **Validation**: Rejects `0` or values `> 65535`. Either `--port` or `--ports` must be provided.

### `--ports <PORTS>`
- **Description**: Comma-separated list or ranges of TCP ports for multi-port listening.
- **Example**: `--ports "80,443,8080-8085"`
- **Constraints**: Maximum range span of 100 ports per item; maximum of 50 total unique listening sockets.

### `-b, --bind <BIND>`
- **Description**: In Daemon mode, the IP address to bind to. In Client mode, the remote destination IP to probe.
- **Default**: `127.0.0.1`.
- **Accepted Values**: Valid IPv4 (e.g., `0.0.0.0`, `192.168.1.10`) or IPv6 address (e.g., `::`, `2001:db8::1`).

### `--admin-token-file <PATH>`
- **Description**: Path to a file containing the secret Bearer token used to authenticate `/metrics` and `/healthz`.
- **Security Check**: Enforces POSIX permissions `0600` or `0400`. Fails startup if readable by group or others.
