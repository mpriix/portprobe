# Contributing to PortProbe

Thank you for your interest in contributing to PortProbe! We welcome bug reports, documentation enhancements, feature proposals, and pull requests from the community.

As an enterprise network verification tool engineered for high reliability and security, PortProbe maintains strict code quality and security standards.

---

## Code of Conduct

All contributors and participants are expected to maintain a welcoming, respectful, inclusive, and harassment-free environment for everyone.

---

## Development Workflow

### 1. Prerequisites

- **Rust**: Version 1.80+ (stable toolchain)
- **Cargo Tools**:
  ```bash
  rustup component add rustfmt clippy
  cargo install cargo-audit cargo-deny
  ```
- **Python**: Version 3.10+ (for building the documentation website)

### 2. Fork & Clone

```bash
git clone https://github.com/<your-username>/portprobe.git
cd portprobe
git checkout -b feature/your-feature-name
```

### 3. Build & Test

```bash
# Build the project
cargo build

# Run the complete test suite (139 tests)
cargo test --all-targets

# Check formatting
cargo fmt --all -- --check

# Run static analysis lints
cargo clippy --all-targets --all-features -- -D warnings

# Run supply chain & license audits
cargo audit
cargo deny check
```

---

## Engineering Guidelines

### 1. Memory Safety & Security Invariants
- **Zero `unsafe` Code**: PortProbe strictly forbids `unsafe` code blocks (`#![deny(unsafe_code)]`). PRs introducing `unsafe` code will not be accepted.
- **Constant-Time Verification**: Any secret or token comparison must use `constant_time_token_verify` to protect against timing attacks.
- **Defensive Error Handling**: Avoid panics, `unwrap()`, or `expect()` in production runtime paths. Return meaningful `Result<T, E>` types.

### 2. Testing Requirements
- Every new feature, CLI option, or bug fix must include corresponding automated tests in `tests/` or unit tests within the relevant module.
- Verify that both IPv4 and IPv6 behaviors are addressed where applicable.
- Ensure that tests clean up temporary files and sockets without leaving background listeners running.

### 3. Documentation
- If your change affects CLI arguments, configuration files, or operational behavior, update the relevant markdown documents under `docs/` and root `README.md`.
- Regenerate the documentation bundle if updating `docs/`:
  ```bash
  python docs/scripts/build.py
  ```

---

## Pull Request Process

1. Ensure your branch is rebased on the latest `main`/`master` branch.
2. Run the pre-submission verification checklist:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-targets
   cargo audit
   cargo deny check
   ```
3. Open a Pull Request on GitHub with a clear description of the problem solved and links to relevant issues.
4. CI checks across Linux, macOS, and Windows must pass before review and merge.

---

## Reporting Vulnerabilities

If you identify a security vulnerability, please do **NOT** open a public issue. Refer to [SECURITY.md](SECURITY.md) for our responsible disclosure procedure.
