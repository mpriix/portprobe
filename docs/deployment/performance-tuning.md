# Performance Tuning & Kernel Optimization

**Document Reference:** `PORTPROBE-DOC-DEP-04`  
**Document Version:** `2.0.0`  
**Classification:** Enterprise Operations Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. High-Concurrency Kernel Tuning (Linux)

To support 5,000 to 10,000 concurrent TCP connections, apply these Linux kernel optimizations in `/etc/sysctl.d/99-portprobe.conf`:

```ini
# Increase maximum system-wide file descriptors
fs.file-max = 2097152

# Increase maximum TCP backlog queue
net.core.somaxconn = 8192
net.ipv4.tcp_max_syn_backlog = 8192

# Enable TCP syncookies for SYN flood protection
net.ipv4.tcp_syncookies = 1

# Optimize TCP socket buffer memory (min, default, max bytes)
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# Accelerate socket recycling for closed connections
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_tw_reuse = 1

# Expand ephemeral port range
net.ipv4.ip_local_port_range = 1024 65535
```

Apply immediately:
```bash
sudo sysctl --system
```

---

## 2. Process File Descriptor Limits (`ulimit`)

Ensure the PortProbe service process has adequate file descriptors:

### In `systemd` (`/etc/systemd/system/portprobe.service`):
```ini
[Service]
LimitNOFILE=65535
```

### In Shell (Ad-Hoc / Terminal Execution):
```bash
ulimit -n 65535
portprobe --port 8080 --max-connections 5000
```

---

## 3. Tokio Runtime Optimization

PortProbe automatically initializes the multi-threaded Tokio runtime with worker threads matching the available CPU core count (`num_cpus`).
- In containerized environments with CPU quotas (e.g., Kubernetes `resources.limits.cpu`), Tokio automatically detects CFS bandwidth limits.
- Memory consumption per active connection averages **4 KB to 8 KB**, meaning 10,000 concurrent sessions require less than 100 MB of total RAM.
