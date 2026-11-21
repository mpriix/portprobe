# PortProbe User Guide Overview

**Document Reference:** `PORTPROBE-DOC-UG-01`  
**Document Version:** `2.0.0`  
**Classification:** Comprehensive User Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Guide Structure & Navigation

The PortProbe User Guide is divided into dedicated modules covering every functional aspect of daemon operations, client probing, network verification, and telemetry:

```
docs/user-guide/
├── overview.md                 <- User guide roadmap and core concepts (this document)
├── daemon-management.md        <- Daemon lifecycle, multi-port binding, signals, privilege drop
├── client-usage.md             <- Active client probe mode (--health-check, TLS, SNI, CA)
├── connectivity-testing.md     <- Interfacing with Netcat, PowerShell, Telnet, curl, Load Balancers
├── concurrency.md              <- Token bucket rate limiting, semaphore connection caps, per-IP limits
├── configuration.md            <- TOML configuration, CLI overrides, environment variable precedence
└── practical-examples.md       <- Real-world enterprise command templates and workflows
```

---

## 2. Core Concepts and Terminology

To make the most of PortProbe, familiarize yourself with these key terms:

- **Daemon Mode**: The default mode of PortProbe. Binds one or more TCP listening sockets, applies admission control, responds with the PortProbe success banner, and records telemetry.
- **Client Probe Mode** (`--health-check`): Client execution mode. PortProbe connects to a remote or local PortProbe listener, validates the success banner, and exits with a standard return code (`0` for success, `1` for failure).
- **Banner**: The deterministic message sent by PortProbe upon establishing a valid TCP session:
  ```text
  PORTPROBE: TCP CONNECTION SUCCESSFUL\n
  ```
- **Silent Rejection** (`--silent-reject`): When an unauthorized client connects (e.g., from an unlisted CIDR subnet or after exhausting rate limits), PortProbe immediately drops the TCP socket without sending an error banner. This prevents automated port scanners (like Nmap or Shodan) from fingerprinting the host.
- **Immediate Reclamation** (`--hold-open false`): The default anti-Slowloris behavior where PortProbe writes its banner and initiates immediate socket shutdown, preventing clients from exhausting connection slots by keeping sockets open.
- **Hold-Open Mode** (`--hold-open true`): Diagnostic mode where PortProbe maintains the TCP connection until `--timeout` expires, allowing engineers to verify firewall state tables and idle session timeouts.
- **Admission Control**: The 5-stage sequential validation pipeline executed on every incoming connection before allocating worker resources (CIDR filter -> Per-IP rate limiter -> Global rate limiter -> Global connection cap -> Per-IP connection cap).
