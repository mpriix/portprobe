# Frequently Asked Questions (FAQ)

**Document Reference:** `PORTPROBE-DOC-TRB-02`  
**Document Version:** `2.0.0`  
**Classification:** Operational Troubleshooting Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. General Questions

### What is PortProbe?
PortProbe is an asynchronous TCP connectivity verification utility written in Rust. It allows infrastructure teams to test network reachability, verify firewall rules, and validate load balancers without deploying full enterprise workloads.

### How does PortProbe differ from Netcat (`nc -l`)?
Netcat lacks concurrency caps, rate limiting, CIDR ingress filtering, structured JSON logging, TLS 1.3/mTLS support, multi-port binding, and administrative telemetry endpoints. Netcat can also be exploited to receive arbitrary payloads or crash servers. PortProbe provides a hardened, enterprise-grade architecture.

### Does PortProbe support UDP?
PortProbe 2.0.0 supports TCP and TLS-over-TCP only. UDP connectivity verification is planned for a future release (see Release Notes).

---

## 2. Security Questions

### Can PortProbe be exploited to execute remote commands?
**No.** PortProbe strictly rejects client payloads. In default mode, it transmits its static banner (`PORTPROBE: TCP CONNECTION SUCCESSFUL\n`) and immediately shuts down the socket. It never parses or executes client input.

### Why does PortProbe drop connections without sending an error banner?
By default, `--silent-reject` is set to `true`. This prevents automated scanners (like Nmap or Shodan) from fingerprinting PortProbe or identifying which ports are actively listening. If you need explicit error banners for debugging, set `--silent-reject false`.

### How does PortProbe authenticate administrative requests?
PortProbe uses constant-time cryptographic Bearer token authentication (`subtle::ConstantTimeEq` combined with SHA-256 pre-hashing), completely mitigating timing attacks.

---

## 3. Operations & Configuration

### How can I reload firewall CIDR rules without restarting PortProbe?
On Linux/Unix systems, edit `/etc/portprobe/portprobe.toml` and send `SIGHUP` to the process:
```bash
sudo systemctl reload portprobe
# or
kill -HUP <PID>
```
PortProbe will reload the CIDR allowlist and TLS certificates in-flight without dropping active connections.

### Why do I get "Too many open files" errors?
Each concurrent connection requires an OS file descriptor. Raise your system limits:
```bash
ulimit -n 65535
```
Or in `systemd`:
```ini
[Service]
LimitNOFILE=65535
```

### Can I run PortProbe as a Windows Service?
Yes. Although native Windows Service Control Manager dispatching (`--service`) is pending in version 2.1.0, you can run PortProbe as a reliable Windows Service today using NSSM (Non-Sucking Service Manager). See [docs/getting-started/installation.md](../getting-started/installation.md).
