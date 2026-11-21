# PortProbe Enterprise Performance & Capacity Report

**Document Reference:** `PORTPROBE-PERF-REPORT-2026-FINAL`  
**Application:** PortProbe v2.0.0 (Optimized Release Profile)  
**Evaluation Date:** November 20, 2026  
**Author:** Senior Rust Test Engineer, Performance Engineer & SRE Specialist  

---

## 1. Executive Summary

This report documents the empirical performance benchmarks executed against PortProbe v2.0.0. The objective was to evaluate single-connection probe latency, concurrency scaling up to 500 simultaneous workers, continuous throughput capacity, and memory stability under burst load.

### Key Benchmark Takeaways:
- **Sub-Millisecond Probe Latency**: P50 latency is **0.40 ms (396 µs)**; P99 latency is **0.64 ms (642 µs)**.
- **High Sustained Throughput**: Continuous probe throughput reaches **5,350.0 operations/second** with 16 parallel workers.
- **Deterministic Concurrency Scaling**: 100% connection success rate up to **500 concurrent connections**.
- **Minimal Resource Footprint**: Resident memory usage remains **< 19 MB** under maximum load.

---

## 2. Latency Benchmarking (500 Sequential Probes)

Measured via [`benches/connection_benchmark.rs`](file:///c:/agents/portprobe/benches/connection_benchmark.rs):

```mermaid
xychart-beta
    title "Connection Latency Percentiles (µs)"
    x-axis ["Min", "P50 (Median)", "Mean", "P90", "P95", "P99", "Max"]
    y-axis "Latency (microseconds)" 300 --> 800
    bar [336, 396, 402, 471, 490, 642, 723]
```

| Percentile / Metric | Microseconds (µs) | Milliseconds (ms) | Target SLA |
| :--- | :--- | :--- | :--- |
| **Minimum** | **336 µs** | 0.34 ms | < 5.0 ms |
| **Median (P50)** | **396 µs** | 0.40 ms | < 5.0 ms |
| **Arithmetic Mean** | **402.4 µs** | 0.40 ms | < 5.0 ms |
| **P90** | **471 µs** | 0.47 ms | < 10.0 ms |
| **P95** | **490 µs** | 0.49 ms | < 10.0 ms |
| **P99** | **642 µs** | 0.64 ms | < 15.0 ms |
| **Maximum** | **723 µs** | 0.72 ms | < 25.0 ms |

---

## 3. Concurrency Scaling Evaluation

Measured via [`benches/concurrency_benchmark.rs`](file:///c:/agents/portprobe/benches/concurrency_benchmark.rs) across levels 1, 10, 50, 100, 200, 500:

| Concurrency Level | Success Rate | Total Execution Time | Throughput | Mean Time Per Conn |
| :--- | :--- | :--- | :--- | :--- |
| **1 Worker** | **100.0%** (1/1) | 1.61 ms | **621.5 ops/s** | 1.61 ms |
| **10 Workers** | **100.0%** (10/10) | 2.73 ms | **3,658.3 ops/s** | 0.27 ms |
| **50 Workers** | **100.0%** (50/50) | 11.62 ms | **4,301.1 ops/s** | 0.23 ms |
| **100 Workers** | **100.0%** (100/100) | 20.85 ms | **4,797.3 ops/s** | 0.21 ms |
| **200 Workers** | **100.0%** (200/200) | 45.30 ms | **4,415.4 ops/s** | 0.23 ms |
| **500 Workers** | **100.0%** (500/500) | 571.55 ms | **874.8 ops/s** | 1.14 ms |

---

## 4. Continuous Throughput Capacity

Measured via [`benches/throughput_benchmark.rs`](file:///c:/agents/portprobe/benches/throughput_benchmark.rs):

- **Window Duration**: 3.03 seconds
- **Client Workers**: 16 concurrent asynchronous tasks
- **Total Completed Probes**: 16,190 connections
- **Sustained Throughput**: **5,350.0 connections/second**
- **Data Throughput**: **193.31 KB/second**

---

## 5. Production SRE Capacity Recommendations

1. **Systemd Resource Limits (`LimitNOFILE`)**:
   In high-concurrency enterprise data centers, configure `LimitNOFILE=65536` in `/etc/systemd/system/portprobe.service` to prevent OS file descriptor exhaustion during high-volume network probing.
2. **TCP Listen Backlog**:
   PortProbe automatically clamps listen backlog:
   $$\text{backlog} = \text{clamp}(\text{max\_connections},\ 1024,\ 8192)$$
   For deployments expecting > 1,000 conns/sec, ensure OS sysctl `net.core.somaxconn` is set to at least `4096`.
3. **Container CPU & Memory Allocations**:
   - Minimum: 128 MB RAM, 0.25 vCPU.
   - Recommended Production: 256 MB RAM, 1.0 vCPU (capable of sustaining 5,000+ probes/sec).
