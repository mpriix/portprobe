# Concurrency, Rate Limiting & Resource Protection

**Document Reference:** `PORTPROBE-DOC-UG-05`  
**Document Version:** `2.0.0`  
**Classification:** Comprehensive User Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Concurrency Controls Overview

Diagnostic tools run on critical network infrastructure must never introduce denial-of-service vulnerabilities. If a misconfigured load balancer, flapping health monitor, or security scanner sends excessive traffic, PortProbe enforces multi-layered admission controls:

| Parameter | CLI Flag | Environment Variable | Default | Valid Range | Operational Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Global Concurrency Cap** | `--max-connections` | `PORTPROBE_MAX_CONNECTIONS` | `100` | 1–10,000 | Limits total simultaneous active connection tasks. |
| **Per-IP Concurrency Cap** | `--max-connections-per-ip` | `PORTPROBE_MAX_CONNECTIONS_PER_IP` | `10` | 1–1,000 | Prevents a single host from consuming all connection slots. |
| **Global Rate Limit** | `--max-rate` | `PORTPROBE_MAX_RATE` | `20` | 1–1,000 req/s | Token-bucket enforcement on total connections per second. |
| **Per-IP Rate Limit** | `--max-rate-per-ip` | `PORTPROBE_MAX_RATE_PER_IP` | `5` | 1–1,000 req/s | Prevents aggressive scanners from starving global rate bucket. |
| **Connection Lifetime** | `--timeout` | `PORTPROBE_TIMEOUT` | `15` | 1–3,600 s | Enforces hard upper bound on connection duration. |

---

## 2. Tuning for Enterprise Scenarios

### Scenario A: High-Concurrency Enterprise Gateway Testing
When testing a large Kubernetes ingress gateway with hundreds of worker nodes connecting simultaneously:
```bash
portprobe \
  --bind 0.0.0.0 \
  --port 8080 \
  --max-connections 5000 \
  --max-connections-per-ip 100 \
  --max-rate 500 \
  --max-rate-per-ip 50
```

> [!IMPORTANT]  
> When configuring `--max-connections` above 500 on Linux, ensure the process file descriptor limit (`ulimit -n`) is elevated to at least `65535`.

### Scenario B: Restrictive Diagnostic Listener (Single-Probe Isolation)
When troubleshooting an isolated point-to-point connection where you wish to strictly prevent background scanner noise:
```bash
portprobe \
  --bind 0.0.0.0 \
  --port 1521 \
  --max-connections 5 \
  --max-connections-per-ip 2 \
  --max-rate 2 \
  --max-rate-per-ip 1 \
  --allow-cidr "10.100.5.20/32" \
  --silent-reject
```

---

## 3. Rate Limiting Behavior & Rejection Responses

### Silent Rejection Mode (`silent_reject = true`, Default):
When an incoming connection violates any rate limit or concurrency cap:
1. PortProbe increments the corresponding metric counter (`rate_limited`, `per_ip_rate_limited`, `connection_limited`, or `per_ip_limited`).
2. The incoming TCP socket is **immediately closed** (`drop(stream)`).
3. The client receives a clean TCP FIN / RST without any data transmission.

### Verbose Rejection Mode (`silent_reject = false`):
If diagnostic error banners are explicitly requested:
- **Global Rate Exceeded**: `PORTPROBE: RATE LIMIT EXCEEDED\n`
- **Global Concurrency Reached**: `PORTPROBE: CONNECTION LIMIT REACHED\n`
- **Per-IP Limit Exceeded**: `PORTPROBE: PER-IP CONNECTION LIMIT REACHED\n`
- **CIDR Blocked**: `PORTPROBE: CONNECTION NOT ALLOWED (CIDR FILTER)\n`
*(Transmitted within a strict 500ms timeout before socket shutdown).*
