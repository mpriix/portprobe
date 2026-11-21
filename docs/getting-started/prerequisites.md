# System & Environment Prerequisites

**Document Reference:** `PORTPROBE-DOC-GS-01`  
**Document Version:** `2.0.0`  
**Classification:** Getting Started Guide  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Supported Operating Systems

PortProbe is engineered for cross-platform enterprise infrastructure. Supported platforms include:

| Operating System | Architecture | Support Tier | Deployment Method | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **Linux** (RHEL 8/9, Rocky, Ubuntu 20.04+, Debian 11+, Alpine 3.18+) | x86_64, aarch64 (ARM64) | **Tier 1 (Production)** | Native Binary, systemd, Container | Full support: privilege dropping (`--user`), SIGHUP hot-reload, POSIX signal handling. |
| **Container / Kubernetes** (Docker, Podman, containerd, OpenShift, EKS, GKE) | Linux x86_64, aarch64 | **Tier 1 (Production)** | OCI Image (via multi-stage Dockerfile) | Non-root UID 10001, native container health check. |
| **Windows Server** (2016, 2019, 2022) & Windows 10/11 | x86_64 | **Tier 2 (Supported)** | Native Binary (`.exe`), Task Scheduler, NSSM | Enforces `set_reuse_address(false)`. Note: `--service` flag is pending native SCM integration. |
| **macOS** (12 Monterey, 13 Ventura, 14 Sonoma, 15 Sequoia) | Apple Silicon (M1/M2/M3/M4), Intel x86_64 | **Development / Testing** | Native Binary (cargo) | Used for local development and diagnostic testing. |

---

## 2. Hardware and Resource Requirements

Due to its zero-garbage-collection Rust architecture, PortProbe has extremely modest hardware requirements:

### Minimum Hardware:
- **CPU**: 1 vCPU (Core 2 Duo, AMD EPYC, AWS Graviton, or newer).
- **RAM**: 64 MB available system RAM.
- **Disk Space**: 20 MB of free storage for binary and log output.

### Recommended Production Hardware (High Concurrency: 1,000+ sessions):
- **CPU**: 2–4 vCPUs.
- **RAM**: 512 MB available system RAM.
- **Network Interface**: 1 Gbps or 10 Gbps Ethernet controller with hardware offloading enabled.

---

## 3. Toolchain & Build Prerequisites (Source Builds)

If building PortProbe from source code, ensure the following developer tools are installed:

### 3.1 Rust Toolchain
- **Rust Version**: `1.80.0` or later (Rust 2021 Edition).
- **Cargo**: Included with official Rust distributions.

To check your current Rust installation:
```bash
rustc --version
cargo --version
```

If Rust is not yet installed on your system, install it via the official installer:
```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Windows (PowerShell)
# Download and execute rustup-init.exe from https://rustup.rs
```

### 3.2 Operating System Build Dependencies
- **Linux (Debian / Ubuntu)**:
  ```bash
  sudo apt-get update && sudo apt-get install -y build-essential pkg-config
  ```
- **Linux (RHEL / Rocky / AlmaLinux)**:
  ```bash
  sudo dnf groupinstall -y "Development Tools"
  ```
- **Alpine Linux**:
  ```bash
  apk add --no-cache musl-dev gcc
  ```
- **Windows**:
  - Microsoft C++ Build Tools (via Visual Studio Installer or Build Tools for Visual Studio 2022).

---

## 4. Network and Firewall Prerequisites

Before starting PortProbe listeners or client probes:

1. **Firewall Ingress Rules**:
   Ensure host firewalls (`ufw`, `firewalld`, `iptables`, Windows Defender Firewall) permit incoming TCP traffic on the intended diagnostic port(s) (e.g., TCP 8080 or the multi-port range configured).
2. **Port Availability**:
   Verify that no other local daemon (Apache, Nginx, PostgreSQL, MySQL) is currently bound to the target port:
   ```bash
   # Linux
   ss -tuln | grep 8080
   # or
   lsof -i :8080

   # Windows (PowerShell)
   Get-NetTCPConnection -LocalPort 8080 -ErrorAction SilentlyContinue
   ```
3. **Privileged Ports Warning (< 1024)**:
   Binding to privileged ports (e.g., 80, 443, 22) requires root privileges on Linux, or granting the `CAP_NET_BIND_SERVICE` Linux capability. PortProbe includes automatic privilege dropping (`--user`, `--group`) to safely drop permissions post-binding.
