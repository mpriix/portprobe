# Build, Test & CI Quality Gates

**Document Reference:** `PORTPROBE-DOC-DEV-02`  
**Document Version:** `2.0.0`  
**Classification:** Developer Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Local Development Workflows

### 1.1 Building
```bash
# Debug build
cargo build

# Optimized release build (thin LTO, codegen-units=1, stripped symbols)
cargo build --release
```

### 1.2 Code Formatting & Linting
```bash
# Format code
cargo fmt --all

# Check formatting in CI
cargo fmt --all -- --check

# Run Clippy with warnings denied
cargo clippy --all-targets --all-features -- -D warnings
```

---

## 2. Test Execution & Regression Suite

PortProbe contains **81 automated unit and integration tests** testing all aspects of the CLI, admission control, and server engines:

### Run All Tests:
```bash
cargo test
```

### Run Tests with Verbose Output:
```bash
cargo test -- --nocapture
```

### Test Suite Structure:
- `cli::tests`: Tests timeout validation, concurrency bounds, CIDR IPv4/IPv6 parsing, dual-stack IPv4-mapped IPv6 subnets, multi-port expansion, port deduplication, config file merging, and CLI override precedence.
- `server::tests`: Tests token-bucket refill calculations, per-IP rate limiting isolation, mutex poisoning recovery, hold-open payload draining caps, silent rejection, SIGHUP CIDR updates, admin server Bearer token validation, Slowloris header caps (4KB), idle connection timeouts, and client health checks.

---

## 3. Security Auditing & Supply Chain Verification

```bash
# Audit dependencies for known vulnerabilities (RustSec)
cargo audit

# Verify licenses, banned crates, and registry sources
cargo deny check

# Generate CycloneDX Software Bill of Materials (SBOM)
cargo cyclonedx --format json --output-pattern "portprobe-sbom.cdx.json"
```

---

## 4. Continuous Integration (CI) Pipeline

PortProbe's GitHub Actions pipeline (`.github/workflows/ci.yml`) enforces five mandatory quality gates:
1. **lint-and-format**: Checks `cargo fmt` and `cargo clippy -D warnings`.
2. **test**: Matrix execution across `ubuntu-latest`, `windows-latest`, and `macos-latest`.
3. **security**: Executes `cargo audit`, `cargo deny check`, and generates CycloneDX SBOM.
4. **container-security**: Builds Alpine container and runs Aquasecurity Trivy scanner, failing on `CRITICAL` or `HIGH` vulnerabilities.
5. **security-regression-gates**: Greps `src/` to verify zero `unsafe` blocks and zero variable-time token comparisons.
6. **release-provenance**: Attests releases via Sigstore Cosign on tagged builds (`v*`).
