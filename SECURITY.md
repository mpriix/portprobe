# Security Policy

The PortProbe maintainers and community take the security of our application and users seriously. As an enterprise network verification utility often deployed within sensitive infrastructure, PortProbe is engineered with defense-in-depth principles.

---

## Supported Versions

Only the latest minor release of PortProbe receives active security patches and vulnerability evaluations.

| Version | Supported          |
| ------- | ------------------ |
| 2.0.x   | :white_check_mark: |
| < 2.0.0 | :x:                |

---

## Reporting a Vulnerability

If you discover a security vulnerability, security flaw, or potential weakness in PortProbe, please **do NOT report it via public GitHub issues, discussions, or pull requests**.

Instead, please report security vulnerabilities responsibly using one of the following methods:

1. **GitHub Private Vulnerability Reporting (Preferred)**:
   Navigate to the **Security** tab of the PortProbe repository on GitHub and click **"Report a vulnerability"** to open a confidential security advisory draft.

2. **Security Contact**:
   Email security advisories directly to the maintainers at `security@portprobe.org` (or via maintainer PGP keys if available).

### What to Include in Your Report

To help us triage and resolve the issue quickly, please provide:
- A clear description of the vulnerability, potential impact, and affected components.
- Step-by-step reproduction instructions, proof-of-concept (PoC) code, or configuration file.
- Any relevant logs, terminal output, or network traces.
- The operating system, architecture, and PortProbe version (`portprobe --version`).

---

## Response & Disclosure Process

1. **Acknowledgment**: We aim to acknowledge receipt of all vulnerability reports within **48 hours**.
2. **Investigation & Triage**: Maintainers will investigate the finding, reproduce the behavior, and determine the severity rating within **5 business days**.
3. **Remediation**: If confirmed, a fix will be developed in a private security advisory branch, validated against the regression test suite, and backported if necessary.
4. **Coordinated Disclosure**: A patched release and CVE advisory (via GitHub Security Advisories) will be published simultaneously, providing reasonable notice and credit to the finder.

---

## Security Architecture & Invariants

PortProbe enforces strict security baselines at compile time and runtime:
- **Zero `unsafe` Code**: Enforced via `#![deny(unsafe_code)]` in `src/lib.rs` and `src/main.rs`.
- **Constant-Time Cryptography**: Administrative Bearer tokens are verified using pre-hashed SHA-256 digests and `subtle::ConstantTimeEq` to eliminate timing side-channel attacks.
- **Input & Range Validation**: All port numbers, timeouts, connection limits, and CIDR subnets are strictly validated prior to operational binding.
- **Memory & Resource Bounding**: Global and per-IP token bucket rate limiters, semaphore concurrency caps, and anti-slowloris socket close timers protect against resource exhaustion DoS.
- **Supply Chain Assurance**: Continuous dependency vulnerability scanning via `cargo-audit` and license enforcement via `cargo-deny`.
