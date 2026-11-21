# PortProbe Release Notes & Changelog

**Document Reference:** `PORTPROBE-DOC-REL-01`  
**Document Version:** `2.0.0`  
**Classification:** Release Specification  
**Status:** Official Release  
**Release Date:** November 2026  

---

## 1. Version 2.0.0 Release Summary

PortProbe 2.0.0 represents a major enterprise release, transitioning the utility from a basic single-port listener into a hardened, high-concurrency, asynchronous connectivity daemon and client probe built on Rust 2021 and Tokio.

### Key Highlights of Version 2.0.0:
- **Multi-Port & Port Range Listening (`--ports`)**: Simultaneous binding across discrete ports and ranges up to 50 listening sockets.
- **Pure-Rust TLS 1.3 & mTLS (`--tls-cert`, `--tls-key`, `--tls-client-ca`)**: Integrated asynchronous TLS engine with client certificate verification and automated certificate expiration tracking.
- **Client Probe Mode (`--health-check`)**: Native client testing mode with TLS SNI verification and standard OS exit codes.
- **Administrative HTTP Server (`--admin-port`)**: Embedded Prometheus `/metrics` and `/healthz` endpoints with constant-time Bearer token authentication.
- **Advanced Admission Control**: Token-bucket rate limiting (global and per-IP), semaphore concurrency caps, and IPv6 `/64` prefix aggregation.
- **Zero-Fingerprint Reconnaissance Protection (`--silent-reject`)**: Drops unauthorized connections immediately at the TCP socket layer without transmitting diagnostic error banners.
- **Anti-Slowloris Architecture (`--hold-open false`)**: Immediate connection slot reclamation post-banner transmission by default.
- **Dynamic Configuration Hot-Reload (`SIGHUP`)**: Live updating of CIDR allowlists and TLS certificates without dropping the daemon.
- **POSIX Privilege Dropping (`--user`, `--group`)**: Securely drops root privileges post-socket-bind.

---

## 2. Comprehensive Changelog

### Version 2.0.0 (Current Release)
- **Engine**: Replaced legacy single-port listener with multi-socket `socket2` architecture.
- **Security**: Added constant-time Bearer token comparison (`subtle::ConstantTimeEq`).
- **Security**: Added POSIX file permission verification (`0600`/`0400`) on keys and secrets.
- **Security**: Hardened Windows sockets with `set_reuse_address(false)`.
- **Telemetry**: Added Prometheus metrics gauge for X.509 certificate expiration (`portprobe_tls_cert_expiry_timestamp_seconds`).
- **Operations**: Added systemd service unit (`portprobe.service`) and hardened Alpine Dockerfile.
- **Observability**: Added structured JSON Lines logging (`--log-format json`) with RFC 3339 local timestamps.
- **Testing**: Expanded automated regression test suite to 81 unit and integration tests.

### Version 1.0.0 (Legacy Baseline)
- Initial single-port TCP listener.
- Basic connection timeout enforcement.
- Plaintext terminal logging.
