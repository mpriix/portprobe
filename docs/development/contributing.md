# Contributing to PortProbe

**Document Reference:** `PORTPROBE-DOC-DEV-03`  
**Document Version:** `2.0.0`  
**Classification:** Developer Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Contribution Principles

We welcome contributions to PortProbe! Before submitting code, please review these guidelines to ensure quality, performance, and security.

### Guiding Principles:
- **No Unsafe Code**: The repository strictly forbids `unsafe` code blocks.
- **Test-Driven Hardening**: Every bug fix, new configuration option, or security patch must include corresponding unit tests in `src/cli.rs` or `src/server.rs`.
- **Backward Compatibility**: Existing CLI flags and TOML configuration parameters must remain stable across minor releases.

---

## 2. Pull Request Submission Checklist

Before submitting a Pull Request (PR):
- [ ] Code passes formatting check: `cargo fmt --all -- --check`
- [ ] Code passes Clippy: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] All automated tests pass: `cargo test --all-targets` (139 tests)
- [ ] Security audit passes: `cargo audit` and `cargo deny check`
- [ ] Zero `unsafe` blocks present in `src/`
- [ ] Documentation updated in `docs/` if modifying flags or behaviors.

---

## 3. Reporting Security Vulnerabilities

If you discover a security vulnerability or potential denial-of-service issue in PortProbe:
- **Do not open a public GitHub issue.**
- Submit details through GitHub Private Vulnerability Reporting or contact the security team directly.
- Include a minimal reproducible test case demonstrating the vulnerability.
