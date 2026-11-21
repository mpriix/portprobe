# PortProbe Configuration Reference

**Document Reference:** `PORTPROBE-DOC-REF-02`  
**Document Version:** `2.0.0`  
**Classification:** Reference Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Configuration Parameter Reference

The table below catalogs every parameter supported in `portprobe.toml`:

| Parameter | Type | Default | Mandatory | Source | Operational Impact |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `port` | Integer | None | Yes (or `ports`) | TOML, CLI, Env | TCP port to bind. If omitted, `ports` must be defined. |
| `ports` | Array of Strings | None | No | TOML, CLI, Env | List of ports and ranges (e.g., `["80", "443", "8080-8085"]`). |
| `bind` | String (IP) | `"127.0.0.1"` | No | TOML, CLI, Env | Interface IP to bind to. Use `"0.0.0.0"` for all IPv4 interfaces. |
| `timeout` | Integer | `15` | No | TOML, CLI, Env | Connection lifetime in seconds (range: 1–3600). |
| `max_connections` | Integer | `100` | No | TOML, CLI, Env | Global concurrent active connection cap (range: 1–10000). |
| `max_connections_per_ip`| Integer | `10` | No | TOML, CLI, Env | Per-source-IP concurrent connection limit (range: 1–1000). |
| `max_rate` | Integer | `20` | No | TOML, CLI, Env | Maximum global connections/sec via token bucket (range: 1–1000).|
| `max_rate_per_ip` | Integer | `5` | No | TOML, CLI, Env | Maximum per-IP connections/sec via token bucket (range: 1–1000).|
| `allow_cidr` | Array of Strings | None | No | TOML, CLI | Subnet CIDR allowlist. Rejects unlisted IPs immediately. |
| `silent_reject` | Boolean | `true` | No | TOML, CLI, Env | Silently drops unauthorized connections without sending error banners.|
| `hold_open` | Boolean | `false` | No | TOML, CLI, Env | If false, closes socket immediately post-banner (anti-Slowloris).|
| `log_format` | String | `"text"` | No | TOML, CLI, Env | Format of emitted logs: `"text"` or `"json"`. |
| `admin_port` | Integer | None | No | TOML, CLI, Env | Localhost port for administrative Prometheus `/metrics` and `/healthz`.|
| `admin_token` | String | None | No | TOML, CLI, Env | Bearer token for admin auth (*prefer `admin_token_file`*). |
| `admin_token_file` | String (Path) | None | No | TOML, CLI, Env | Path to file containing Bearer token with 0600 permissions. |
| `tls_min_version` | String | `"1.2"` | No | TOML, CLI, Env | Minimum TLS protocol version: `"1.2"` or `"1.3"`. |
| `tls_cert` | String (Path) | None | No | TOML, CLI, Env | Path to server X.509 certificate PEM file. |
| `tls_key` | String (Path) | None | No | TOML, CLI, Env | Path to server private key PEM file (enforces 0600 permissions). |
| `tls_client_ca` | String (Path) | None | No | TOML, CLI, Env | Path to CA bundle for mutual TLS (mTLS) client validation. |
| `user` | String | None | No | TOML, CLI, Env | POSIX user account to drop privileges to post-socket-bind. |
| `group` | String | None | No | TOML, CLI, Env | POSIX group account to drop privileges to post-socket-bind. |

---

## 2. Complete Enterprise Production Configuration Example

```toml
# ==============================================================================
# PortProbe Enterprise Production Configuration Template
# Path: /etc/portprobe/portprobe.toml
# Permissions: 0640 | Owner: root:portprobe
# ==============================================================================

# Core Listening Sockets
port = 8080
ports = ["8081", "8082", "8443", "9000-9005"]
bind = "0.0.0.0"

# Timing and Concurrency Bounds
timeout = 30
max_connections = 2000
max_connections_per_ip = 50
max_rate = 100
max_rate_per_ip = 10

# Ingress Network Filtering
allow_cidr = [
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "127.0.0.1/32"
]

# Security Controls
silent_reject = true
hold_open = false
log_format = "json"

# Administrative Metrics & Healthcheck
admin_port = 9090
admin_token_file = "/etc/portprobe/admin_token.secret"

# Transport Encryption (TLS 1.3 Strict)
tls_cert = "/etc/portprobe/certs/server.crt"
tls_key = "/etc/portprobe/certs/server.key"
tls_min_version = "1.3"
tls_client_ca = "/etc/portprobe/certs/ca-bundle.crt"

# Linux Privilege Dropping
user = "portprobe"
group = "portprobe"
```
