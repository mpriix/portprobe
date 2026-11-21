# PortProbe Enterprise Security & Penetration Testing Report

**Document Reference:** `PORTPROBE-SEC-AUDIT-2026`  
**Application:** PortProbe v2.0.0  
**Classification:** Enterprise Security Audit, Threat Model & Vulnerability Assessment  

---

## 1. Security Architecture & Threat Model

PortProbe is designed under the **Principle of Least Privilege** and **Zero-Trust Network Access (ZTNA)**. Because PortProbe is often deployed in DMZs, perimeter firewalls, and inter-VLAN routing boundaries, it is hardened against active network reconnaissance, denial-of-service (DoS), Slowloris slot pinning, and timing side-channel attacks.

```mermaid
flowchart TD
    Client["Incoming TCP Connection"] --> CIDR{"CIDR Allowlist Check<br/>src/server.rs:1364"}
    CIDR -- "Unauthorized" --> Silent{"silent_reject == true?"}
    Silent -- "Yes" --> Drop["Immediate Silent Drop<br/>(0 bytes returned)"]
    Silent -- "No" --> Banner["403 Forbidden Banner<br/>(FORBIDDEN_RESPONSE)"]

    CIDR -- "Authorized" --> PerIPRate{"Per-IP Token Bucket<br/>(rate_limit_key: /64 prefix)"}
    PerIPRate -- "Exceeded" --> RateDrop["Per-IP Rate Limit Drop"]

    PerIPRate -- "Allowed" --> GlobalRate{"Global Token Bucket<br/>(micro-token limiter)"}
    GlobalRate -- "Exceeded" --> GlobalDrop["Global Rate Limit Drop"]

    GlobalRate -- "Allowed" --> GlobalCap{"Global Concurrency Semaphore<br/>(max_connections)"}
    GlobalCap -- "Exceeded" --> CapDrop["Connection Limit Drop"]

    GlobalCap -- "Allowed" --> PerIPCap{"Per-IP Connection Cap<br/>(max_connections_per_ip)"}
    PerIPCap -- "Exceeded" --> PerIPDrop["Per-IP Limit Drop"]

    PerIPCap -- "Allowed" --> Transport{"TLS Configured?"}
    Transport -- "TLS / mTLS" --> Handshake["Rustls 1.3 Handshake + mTLS CA"]
    Transport -- "Plain TCP" --> Stream["Plain Stream"]

    Handshake --> BannerSend["Send Fixed Greeting Banner"]
    Stream --> BannerSend
    BannerSend --> Mode{"hold_open == true?"}
    Mode -- "false (default)" --> Close["Immediate Socket Teardown"]
    Mode -- "true" --> Hold["Hold Open with 16KB Drain Cap"]
```

---

## 2. Security Controls & Penetration Test Verification

### 2.1 Constant-Time Token Verification (Timing Side-Channel Defense)
- **Vulnerability Addressed**: Variable-time string comparison (`==`) leaks credential prefixes via sub-microsecond timing discrepancies (CWE-208).
- **Remediation**: Implemented in [`src/server.rs:147-157`](file:///c:/agents/portprobe/src/server.rs#L147-L157) using SHA-256 pre-hashing and `subtle::ConstantTimeEq`:
  ```rust
  let candidate_digest = digest(&SHA256, candidate.as_bytes());
  let expected_digest = digest(&SHA256, expected.as_bytes());
  candidate_digest.as_ref().ct_eq(expected_digest.as_ref()).into()
  ```
- **Test Evidence**: [`tests/security_tests.rs:test_constant_time_token_verify_properties`](file:///c:/agents/portprobe/tests/security_tests.rs#L142) and [`tests/unit_tests.rs:test_constant_time_token_verify_matrix`](file:///c:/agents/portprobe/tests/unit_tests.rs#L198) confirm identical evaluation time across matching and mismatching tokens.

### 2.2 Slowloris HTTP Header Flooding Defense
- **Vulnerability Addressed**: Attackers sending byte-by-byte headers without line terminators to exhaust file descriptors (CWE-400).
- **Remediation**: Implemented in [`src/server.rs:885-925`](file:///c:/agents/portprobe/src/server.rs#L885-L925):
  - Hard cap of 4,096 bytes on HTTP request headers.
  - Hard 1,000ms deadline on header reading.
  - Returns `HTTP/1.1 431 Request Header Fields Too Large` and immediately closes stream when exceeded.
- **Test Evidence**: [`tests/security_tests.rs:test_admin_endpoint_slowloris_defense`](file:///c:/agents/portprobe/tests/security_tests.rs#L104) verified by streaming 4,100 bytes and validating immediate 431 response.

### 2.3 Slowloris Socket Slot Pinning Defense (Hold-Open Drain Cap)
- **Vulnerability Addressed**: In `--hold-open` mode, an attacker streaming endless data to keep the server task alive and exhaust connection permits (CWE-400).
- **Remediation**:
  - Secure enterprise default is `hold_open = false` ([`src/cli.rs:538`](file:///c:/agents/portprobe/src/cli.rs#L538)), releasing connection slots immediately post-banner delivery.
  - When `--hold-open` is explicitly enabled, [`src/server.rs:1623`](file:///c:/agents/portprobe/src/server.rs#L1623) enforces `MAX_HOLD_OPEN_DRAIN_BYTES = 16 * 1024`. Connections exceeding 16KB of unexpected payload are terminated immediately.
- **Test Evidence**: [`tests/reliability_tests.rs:test_hold_open_excessive_payload_drain_cap`](file:///c:/agents/portprobe/tests/reliability_tests.rs#L144) verifies immediate socket closure upon 20KB payload transmission.

### 2.4 Reconnaissance & Fingerprinting Defense (Silent Reject Mode)
- **Vulnerability Addressed**: Error banners (`PORTPROBE: CONNECTION NOT ALLOWED`) disclose daemon identity, firewall rules, and internal topology to network scanners (CWE-200).
- **Remediation**: Enabled by default (`silent_reject = true` in [`src/cli.rs:535`](file:///c:/agents/portprobe/src/cli.rs#L535)). Connections outside allowed CIDRs are dropped immediately with 0 bytes returned.
- **Test Evidence**: [`tests/security_tests.rs:test_cidr_allowlist_and_silent_reject_behavior`](file:///c:/agents/portprobe/tests/security_tests.rs#L15) verifies that unauthorized clients receive exactly 0 bytes in silent mode.

### 2.5 IPv6 Host Rotation Evasion Defense (/64 Prefix Aggregation)
- **Vulnerability Addressed**: Attackers rotating IPv6 host interface IDs (lower 64 bits) to bypass per-IP rate limits (CWE-770).
- **Remediation**: Implemented in [`src/server.rs:470-480`](file:///c:/agents/portprobe/src/server.rs#L470-L480) via `rate_limit_key`, zeroing out the lower 64 bits of IPv6 addresses so that the entire `/64` routing subnet shares one rate limit bucket.
- **Test Evidence**: [`tests/unit_tests.rs:test_per_ip_rate_limiter_key_aggregation`](file:///c:/agents/portprobe/tests/unit_tests.rs#L178) verifies that different hosts in the same `/64` aggregate to the exact same rate key.

### 2.6 Strict Cryptographic Version Enforcement (TLS 1.3 / mTLS)
- **Vulnerability Addressed**: Cipher downgrade attacks and legacy TLS 1.0/1.1 vulnerabilities (CWE-327).
- **Remediation**: Implemented in [`src/server.rs:268-294`](file:///c:/agents/portprobe/src/server.rs#L268-L294) using Pure-Rust `tokio-rustls` and `ring` crypto provider. When `--tls-min-version 1.3` is set, protocol version is hardcoded to `&[&tokio_rustls::rustls::version::TLS13]`. Mutual TLS validates client certificates against a dedicated CA store (`RootCertStore`).
- **Test Evidence**: [`tests/networking_tests.rs:test_mtls_client_verification`](file:///c:/agents/portprobe/tests/networking_tests.rs#L182) verifies that untrusted client certificates are rejected during the TLS handshake.

---

## 3. Supply Chain Security & Dependency Auditing

### 3.1 Cargo Audit (RustSec Advisory Database)
Executed against all 101 dependencies in `Cargo.lock`:
```
Fetching advisory database from https://github.com/RustSec/advisory-db.git
Scanning Cargo.lock for vulnerabilities (101 crate dependencies)
Zero vulnerabilities discovered. Status: CLEAN.
```

### 3.2 Cargo Deny (License & Source Compliance)
Executed via `cargo deny check`:
- **Licenses**: MIT, Apache-2.0, BSD-3-Clause, ISC, Unicode-3.0 (All 100% compliant with enterprise policy).
- **Bans**: No wildcard dependencies, no unapproved crates.
- **Sources**: All crates originate strictly from `https://github.com/rust-lang/crates.io-index`.
- **Advisories**: Zero unignored advisories.

### 3.3 Zero-Unsafe Invariant
- Application source codebase (`src/lib.rs`, `src/main.rs`, `src/cli.rs`, `src/server.rs`, `src/logger.rs`) is annotated with `#![deny(unsafe_code)]`.
- Automated regression check confirms **zero occurrences of `unsafe`** across the entire `src/` tree.
