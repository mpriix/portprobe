# PortProbe Test Execution & Operations Guide

**Document Reference:** `PORTPROBE-EXEC-GUIDE-2026`  
**Application:** PortProbe v2.0.0  

---

## 1. Prerequisites & Toolchain Setup

To execute the complete PortProbe test, benchmarking, and quality assurance suite, ensure the following tools are available:

- **Rust Toolchain**: `stable` (1.80+ or 1.100+ recommended)
- **Rust Components**: `rustfmt`, `clippy`, `llvm-tools-preview`
  ```powershell
  rustup component add rustfmt clippy llvm-tools-preview
  ```
- **Cargo Utilities**: `cargo-audit`, `cargo-deny`, `cargo-llvm-cov`
  ```powershell
  cargo install cargo-audit cargo-deny cargo-llvm-cov --locked
  ```
- **OpenSSL**: Version 3.0+ (required for regenerating test X.509 certs if needed)

---

## 2. Executing Automated Tests

### 2.1 Run All Tests (139 Test Cases)
```powershell
cargo test --all-targets
```

### 2.2 Run Specific Test Suites
```powershell
# Run Internal Unit Tests in lib.rs (84 tests)
cargo test --lib

# Run External Unit Tests (13 tests)
cargo test --test unit_tests

# Run Integration & Lifecycle Tests (4 tests)
cargo test --test integration_tests

# Run TCP Networking & TLS Tests (9 tests)
cargo test --test networking_tests

# Run High-Concurrency & Admission Control Tests (4 tests)
cargo test --test concurrency_tests

# Run Configuration Loading & Precedence Tests (9 tests)
cargo test --test configuration_tests

# Run CLI Execution & Exit Code Tests (9 tests)
cargo test --test cli_tests

# Run Reliability, Stability & Resource Tests (4 tests)
cargo test --test reliability_tests

# Run Security, Slowloris & Access Control Tests (3 tests)
cargo test --test security_tests
```

### 2.3 Run a Single Targeted Test with Console Output
```powershell
cargo test --test cli_tests -- test_cli_healthcheck_success_exit_code --nocapture
```

---

## 3. Static Analysis & Lints

### 3.1 Code Formatting Validation
```powershell
cargo fmt --all -- --check
```

### 3.2 Strict Clippy Lints (Zero Warnings Enforced)
```powershell
cargo clippy --all-targets --all-features -- -D warnings
```

---

## 4. Running Performance Benchmarks

All benchmarks are located in [`benches/`](file:///c:/agents/portprobe/benches/) and compile under the optimized `bench` profile.

### 4.1 Connection Latency Benchmark (500 samples, P50/P95/P99)
```powershell
cargo bench --bench connection_benchmark
```

### 4.2 Concurrency Scaling Benchmark (1, 10, 50, 100, 200, 500 workers)
```powershell
cargo bench --bench concurrency_benchmark
```

### 4.3 Sustained Throughput Benchmark (16 workers, 3-second window)
```powershell
cargo bench --bench throughput_benchmark
```

---

## 5. Generating Code Coverage Reports

Using `cargo-llvm-cov`:

### 5.1 Terminal Summary Report
```powershell
cargo llvm-cov --lib --summary-only
```

### 5.2 HTML Detailed Coverage Report
```powershell
cargo llvm-cov --lib --html
# Generates report in target/llvm-cov/html/index.html
```

---

## 6. Security & Dependency Auditing

### 6.1 Vulnerability Scan via RustSec
```powershell
cargo audit
```

### 6.2 Supply Chain Policy & License Compliance
```powershell
cargo deny check
```
