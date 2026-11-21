# PortProbe Developer Architecture Guide

**Document Reference:** `PORTPROBE-DOC-DEV-01`  
**Document Version:** `2.0.0`  
**Classification:** Developer Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Codebase Organization

The PortProbe repository follows standard Rust idiomatic structure:

```text
portprobe/
├── Cargo.toml            <- Dependencies, features, and release optimization profile
├── Cargo.lock            <- Locked dependency graph (101 dependencies)
├── deny.toml             <- cargo-deny policy (licenses, bans, advisories, sources)
├── Dockerfile            <- Multi-stage hardened Alpine container definition
├── portprobe.service     <- Production systemd service unit
├── portprobe.toml        <- Canonical configuration template
├── src/
│   ├── main.rs           <- Entry point, runtime bootstrap, signal management
│   ├── cli.rs            <- Clap argument definitions, validators, CIDR parser
│   ├── server.rs         <- TCP listener, admission control, TLS engine, admin HTTP
│   └── logger.rs         <- Tracing subscriber, formatters, RFC 3339 timezone
└── docs/                 <- Complete enterprise product documentation
```

---

## 2. Core Dependencies & Technology Stack

| Dependency | Version | Purpose in PortProbe |
| :--- | :--- | :--- |
| `tokio` | 1.x | Asynchronous runtime (`rt-multi-thread`, `net`, `time`, `signal`, `sync`). |
| `clap` | 4.x | Command-line argument parsing with derive macros and environment variable support. |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 | Structured diagnostic event logging with JSON Lines formatting and env filtering. |
| `tokio-rustls` / `rustls-pki-types` | 0.26 / 1.x | Pure-Rust TLS 1.3 / 1.2 asynchronous encryption engine without OpenSSL C-bindings. |
| `socket2` | 0.6 | Low-level OS socket creation, dual-stack IPv6, and `SO_REUSEADDR` configuration. |
| `subtle` | 2.6 | Constant-time cryptographic comparison primitives (`ConstantTimeEq`). |
| `ring` | 0.17 | Cryptographic SHA-256 digest computation for constant-time token hashing. |
| `serde` / `toml` | 1.x / 0.8 | Deserialization of declarative TOML configuration files. |
| `time` | 0.3 | Thread-safe RFC 3339 local timezone timestamp formatting. |
| `privdrop` / `nix` | 0.5 / 0.30 | Safe POSIX privilege dropping and EUID inspection on Unix systems. |

---

## 3. Coding Conventions & Invariants

All contributions must strictly adhere to the following architectural invariants:

1. **Zero Unsafe Code**: `#![deny(unsafe_code)]` is enforced at the crate root in `src/main.rs`. PRs containing `unsafe` blocks will be rejected by automated CI compiler gates.
2. **Deterministic Cryptography**: Never use variable-time string comparisons (`==`) when evaluating authentication tokens. Always hash candidate strings with SHA-256 and evaluate with `subtle::ConstantTimeEq`.
3. **Mutex Poisoning Resilience**: Never call `.unwrap()` directly on a standard `Mutex` lock in long-lived server code. Always handle poisoned locks gracefully via `.unwrap_or_else(|p| p.into_inner())`.
4. **Error Handling**: Use `Result<T, String>` or `Result<T, Box<dyn std::error::Error + Send + Sync>>`. Return `std::process::ExitCode::SUCCESS` or `ExitCode::FAILURE` from `main()`.
5. **Logging Levels**:
   - `error!`: Unrecoverable errors that terminate the daemon or invalidate configuration.
   - `warn!`: Significant security events (TLS handshake failure, privileged port warnings, deprecated CLI token usage).
   - `info!`: Connection lifecycle events (started, accepted, success response sent, shutdown).
   - `debug!`: Granular diagnostics (CIDR drops, rate limit drops, EOF detections, byte counts).
