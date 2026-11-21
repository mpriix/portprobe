# Error Reference & Diagnostic Codes

**Document Reference:** `PORTPROBE-DOC-REF-03`  
**Document Version:** `2.0.0`  
**Classification:** Reference Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Startup and Configuration Errors

| Error Message | Origin | Root Cause | Remediation |
| :--- | :--- | :--- | :--- |
| `Configuration error: Port must be between 1 and 65535` | `src/cli.rs` | Port `0` or negative/invalid number was passed to `--port` or in `--ports`. | Specify a valid TCP port between 1 and 65535. |
| `Configuration error: Timeout must be between 1 and 3600 seconds` | `src/cli.rs` | `--timeout` passed value outside 1–3600. | Set timeout within allowable operational range. |
| `Configuration error: Per-IP connection limit (X) cannot exceed global limit (Y)` | `src/cli.rs` | `--max-connections-per-ip` was set higher than `--max-connections`. | Set per-IP concurrency less than or equal to global concurrency. |
| `Configuration error: Per-IP rate limit (X) cannot exceed global rate limit (Y)` | `src/cli.rs` | `--max-rate-per-ip` was set higher than `--max-rate`. | Set per-IP rate limit less than or equal to global rate. |
| `Configuration error: Total number of listening ports (X) exceeds maximum limit of 50` | `src/cli.rs` | Expanded port ranges exceeded 50 unique ports. | Reduce port ranges to 50 sockets or fewer. |
| `Configuration error: Insecure permissions on <path>: X. Must not be accessible by group or others (enforce 0600 or 0400)` | `src/cli.rs` | Secret key or admin token file has group/world read bits. | Execute `chmod 0600 <path>`. |
| `Configuration error: Both --tls-cert and --tls-key must be specified together for TLS` | `src/cli.rs` | `--tls-cert` was provided without `--tls-key` or vice versa. | Specify both certificate and private key together. |
| `Configuration error: Invalid tls_min_version 'X'. Allowed values: '1.2' or '1.3'` | `src/cli.rs` | Unsupported TLS version requested. | Specify `"1.2"` or `"1.3"`. |
| `Failed to bind administrative endpoint on 127.0.0.1:X: Address already in use` | `src/server.rs` | Port conflict on administrative port. | Select an unused port for `--admin-port`. |
| `CRITICAL: Running as root (EUID 0) but failed to drop privileges to user X, group Y` | `src/server.rs` | Daemon started as root but target user/group does not exist. | Create the user/group or ensure correct permissions. |

---

## 2. Network Banners (Verbose Mode: `silent_reject = false`)

When `silent_reject = false`, PortProbe emits explicit ASCII banners:

| Banner String | Cause | Client Action |
| :--- | :--- | :--- |
| `PORTPROBE: TCP CONNECTION SUCCESSFUL\n` | Connection admitted and completed. | Reachability verified. |
| `PORTPROBE: RATE LIMIT EXCEEDED\n` | Global connection rate exceeded `--max-rate`. | Backoff connection attempts. |
| `PORTPROBE: PER-IP CONNECTION LIMIT REACHED\n` | Source IP active connections exceeded `--max-connections-per-ip`. | Close existing active connections. |
| `PORTPROBE: CONNECTION LIMIT REACHED\n` | Global concurrent connections exceeded `--max-connections`. | Scale server concurrency or backoff. |
| `PORTPROBE: CONNECTION NOT ALLOWED (CIDR FILTER)\n` | Source IP is not contained within `--allow-cidr`. | Add source IP subnet to CIDR allowlist. |

---

## 3. Administrative HTTP Status Codes (`--admin-port`)

| HTTP Code | Endpoint | Description |
| :--- | :--- | :--- |
| `200 OK` | `/healthz`, `/metrics` | Request successful. |
| `401 Unauthorized` | `/healthz`, `/metrics` | Missing or invalid `Authorization: Bearer <token>` header. |
| `404 Not Found` | Any other URI | Requested path does not exist on administrative server. |
| `429 Too Many Requests`| Any URI | Administrative rate limit (20 req/s) exceeded. |
| `431 Request Header Fields Too Large`| Any URI | HTTP request headers exceeded 4,096 bytes (Slowloris mitigation). |
| `503 Service Unavailable`| `/metrics` | Metrics concurrency limit (12 permits) reached. |
