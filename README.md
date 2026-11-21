# PortProbe

**Enterprise Asynchronous TCP Connectivity Verification Daemon and Client Probe**

[![CI Status](https://github.com/portprobe/portprobe/actions/workflows/ci.yml/badge.svg)](https://github.com/portprobe/portprobe/actions)
[![Rust Version](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-2.0.0-blue.svg)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Security Policy](https://img.shields.io/badge/security-NIST%20SP%20800--53-red.svg)](docs/security/security-guide.md)

PortProbe is an asynchronous, high-concurrency TCP connectivity verification daemon and probe utility written in 100% safe Rust. Designed specifically for Site Reliability Engineering (SRE), DevOps, Cloud Infrastructure, and Network Security teams, PortProbe validates network reachability, verifies firewall policies, tests load balancer health probes, and diagnoses distributed routing paths **without requiring full enterprise multi-tier applications to be deployed**.

---

## Key Capabilities at a Glance

- **Multi-Port & Port Range Binding**: Listen across discrete ports and continuous ranges (e.g., `80,443,8080-8085`, up to 50 listening sockets) simultaneously on IPv4 and IPv6 interfaces.
- **Dual Operating Modes**:
  - **Daemon Mode**: High-performance TCP listener returning standard verification banners (`PORTPROBE: TCP CONNECTION SUCCESSFUL\n`).
  - **Client Probe Mode** (`--health-check`): Active client prober that connects to a target host/port, performs banner validation, and exits with standard process exit codes (0 for success, 1 for failure).
- **Enterprise Transport Security**: Native TLS 1.3/1.2 encryption powered by pure-Rust `rustls`, including Mutual TLS (mTLS) client certificate verification, dynamic SIGHUP certificate hot-reloading, and automated X.509 certificate expiration alerting.
- **Defense-in-Depth Admission Control**:
  - Token-bucket global rate limiting (1–1,000 req/s).
  - Per-source-IP token-bucket rate limiting (1–1,000 req/s) with IPv6 `/64` prefix aggregation.
  - Global semaphore connection concurrency caps (1–10,000 active sessions).
  - Per-source-IP concurrency limits (1–1,000 active sessions).
  - CIDR subnet ingress filtering with zero-overhead IP parsing.
  - Anti-reconnaissance silent rejection (`--silent-reject`) to prevent port scanning finger-printing.
- **Anti-Slowloris Architecture**: Immediate connection slot reclamation post-banner transmission by default, with optional configurable hold-open mode (`--hold-open`).
- **Production Observability**:
  - Structured JSON logging (`--log-format json`) for SIEM platforms (Splunk, Datadog, Elastic) with RFC 3339 local timestamps.
  - Dedicated administrative server (`--admin-port`) providing Prometheus `/metrics` and HTTP `/healthz` endpoints protected by constant-time Bearer token authentication.
- **Hardened System Integration**:
  - POSIX privilege dropping (`--user`, `--group`) post-socket-bind.
  - Production-ready `systemd` service unit with strict sandboxing (`NoNewPrivileges`, `ProtectSystem=strict`).
  - Minimal Alpine container with non-root UID 10001 execution and native container health checks.

---

## Architecture Overview

```mermaid
flowchart TD
    subgraph Client Infrastructure
        LB[Load Balancers & Probes]
        SRE[SRE & DevOps Engineers]
        CI[CI/CD Pipelines]
    end

    subgraph PortProbe Daemon
        L1[TCP Listener :80]
        L2[TCP Listener :443]
        L3[TCP Listener :8080-8085]
        
        AC{Admission Control<br/>CIDR & Rate Limiters}
        TLS[TLS 1.3 / mTLS Engine]
        CH[Connection Handler<br/>Tokio Async Tasks]
        
        ADMIN[Admin Server :9090<br/>GET /healthz | GET /metrics]
    end

    subgraph Enterprise Infrastructure
        PROM[Prometheus / Grafana]
        SIEM[SIEM / Splunk / Elastic]
    end

    LB -->|TCP Probe| L1
    SRE -->|Connectivity Verification| L2
    CI -->|portprobe --health-check| L3

    L1 --> AC
    L2 --> AC
    L3 --> AC

    AC -->|Drop / Silent Reject| DROP[Connection Dropped]
    AC -->|Admitted| TLS
    TLS --> CH
    CH -->|PORTPROBE Banner| SRE

    ADMIN -->|Scrape /metrics| PROM
    PortProbe Daemon -.->|JSON Logs| SIEM
```

---

## Quick Start

### 1. Installation

#### Build from Source (Requires Rust 1.80+)
```bash
git clone https://github.com/portprobe/portprobe.git
cd portprobe
cargo build --release
sudo cp target/release/portprobe /usr/local/bin/
```

#### Run with Docker
```bash
docker build -t portprobe:2.0.0 .
docker run -d --name portprobe -p 8080:8080 -e PORTPROBE_BIND=0.0.0.0 portprobe:2.0.0
```

### 2. Start a Connectivity Verification Daemon

Start a listener on port 8080 with CIDR filtering and administrative metrics enabled:

```bash
portprobe \
  --bind 0.0.0.0 \
  --port 8080 \
  --allow-cidr "10.0.0.0/8,192.168.1.0/24" \
  --admin-port 9090 \
  --log-format json
```

### 3. Verify Reachability from a Client

#### Using PortProbe Built-in Client Healthcheck
```bash
portprobe --health-check --bind 10.0.0.50 --port 8080
```
Output:
```text
PortProbe healthcheck SUCCESS: 10.0.0.50:8080 is reachable
```

#### Using Standard Network Tools
```bash
# Using Netcat
nc -vz 10.0.0.50 8080

# Using PowerShell
Test-NetConnection -ComputerName 10.0.0.50 -Port 8080
```

---

## Complete Documentation Index

PortProbe includes an exhaustive enterprise documentation suite located in the [`docs/`](docs/) directory:

| Section | Documentation Link | Description |
| :--- | :--- | :--- |
| **Product Overview** | [docs/product/product-overview.md](docs/product/product-overview.md) | Vision, problem statement, core components, and competitive analysis |
| **Architecture & Design** | [docs/product/architecture.md](docs/product/architecture.md) | Component architecture, execution model, concurrency, and security |
| **Technical Design** | [docs/product/technical-design.md](docs/product/technical-design.md) | Internal module mechanics, data structures, and state transitions |
| **Getting Started** | [docs/getting-started/quick-start.md](docs/getting-started/quick-start.md) | Step-by-step installation, prerequisites, and first connectivity probe |
| **Comprehensive User Guide** | [docs/user-guide/overview.md](docs/user-guide/overview.md) | Complete operational guide covering every daemon and client capability |
| **CLI Reference** | [docs/reference/cli-reference.md](docs/reference/cli-reference.md) | Exhaustive parameter reference, environment variables, and syntax |
| **Configuration Reference** | [docs/reference/configuration-reference.md](docs/reference/configuration-reference.md) | TOML configuration file parameters, schemas, and precedence rules |
| **Real-World Scenarios** | [docs/examples/firewall-validation.md](docs/examples/firewall-validation.md) | Enterprise walkthroughs: Firewalls, Load Balancers, CI/CD pipelines |
| **Deployment & Ops** | [docs/deployment/deployment-guide.md](docs/deployment/deployment-guide.md) | systemd service setup, container orchestration, and performance tuning |
| **Troubleshooting & FAQ** | [docs/troubleshooting/troubleshooting-guide.md](docs/troubleshooting/troubleshooting-guide.md) | Diagnostic workflows, common error resolutions, and operational FAQ |
| **Security Architecture** | [docs/security/security-guide.md](docs/security/security-guide.md) | NIST/CIS compliance, memory safety, privilege dropping, and mTLS |
| **Developer Guide** | [docs/development/developer-guide.md](docs/development/developer-guide.md) | Repository organization, compilation workflows, and contribution guide |
| **Release Notes & Roadmap** | [docs/release-notes/changelog.md](docs/release-notes/changelog.md) | Version history, implemented feature matrix, and enterprise roadmap |
| **Audit Closure Report** | [docs/DOCUMENTATION_AUDIT_REPORT.md](docs/DOCUMENTATION_AUDIT_REPORT.md) | Detailed documentation and technical findings closure verification |

---

## Configuration Example (`portprobe.toml`)

```toml
# PortProbe Production Configuration
port = 8080
ports = ["8081", "8082", "8090-8095"]
bind = "0.0.0.0"
timeout = 15
max_connections = 1000
max_connections_per_ip = 25
max_rate = 100
max_rate_per_ip = 10
allow_cidr = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"]
silent_reject = true
hold_open = false
log_format = "json"

# Administrative Health and Prometheus Metrics
admin_port = 9090
admin_token_file = "/etc/portprobe/admin_token.secret"

# TLS 1.3 Encryption
tls_cert = "/etc/portprobe/certs/server.crt"
tls_key = "/etc/portprobe/certs/server.key"
tls_min_version = "1.3"
```

---

## Security & Verification Baseline

- **Zero Unsafe Blocks**: Enforced via `#![deny(unsafe_code)]` at compile-time.
- **Deterministic Cryptography**: Constant-time Bearer token verification using SHA-256 pre-hashing and `subtle::ConstantTimeEq`.
- **Automated Regression Suite**: 139 comprehensive unit, integration, concurrency, and security tests passing in CI across Linux, macOS, and Windows.
- **Vulnerability Scanning**: Automated Trivy container scans and `cargo-audit` advisory checks on every commit.
- **Security Policy**: See [SECURITY.md](SECURITY.md) for vulnerability disclosure guidelines.

---

## Contributing

We welcome contributions! Please review our [Contribution Guidelines](CONTRIBUTING.md) before opening a pull request or submitting an issue.

---

## License

PortProbe is released under the [MIT License](LICENSE).
