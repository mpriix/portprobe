# Daemon Management & Lifecycle Guide

**Document Reference:** `PORTPROBE-DOC-UG-02`  
**Document Version:** `2.0.0`  
**Classification:** Comprehensive User Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Starting the Daemon

The PortProbe daemon is started simply by invoking `portprobe` with the desired port or configuration options.

### 1.1 Single Port Listening
To listen on a single port (e.g., port `8080`):
```bash
portprobe --port 8080
```
By default, PortProbe binds exclusively to `127.0.0.1` (localhost) to ensure security.

### 1.2 Listening on All Network Interfaces
To accept incoming connections across all physical and virtual interfaces (e.g., for remote firewall testing):
```bash
portprobe --bind 0.0.0.0 --port 8080
```

> [!WARNING]  
> When `--bind 0.0.0.0` or `--bind ::` is used, PortProbe logs a prominent warning:  
> `Listening on all interfaces. Ensure firewall restrictions are configured.`  
> Always combine wildcard bindings with `--allow-cidr` or external firewall rules.

---

## 2. Multi-Port and Port Range Listening

PortProbe supports binding multiple discrete ports and continuous port ranges simultaneously in a single daemon process.

### Syntax: `--ports <PORTS>`
The `--ports` argument accepts a comma-separated list of individual port numbers and port ranges:

```bash
portprobe --bind 0.0.0.0 --ports "80,443,8080-8085,9000"
```

### Safety Rules & Validation:
1. **Range Bounds**: Port numbers must be between `1` and `65535`.
2. **Range Direction**: The start port must be less than or equal to the end port (`8080-8085` is valid; `8085-8080` is rejected).
3. **Range Span Cap**: A single range cannot exceed 100 ports (`end - start <= 100`).
4. **Global Port Limit**: The total number of unique listening sockets cannot exceed **50 ports**.
5. **Deduplication**: If overlapping ports are specified (e.g., `--port 8080 --ports "8080-8082"`), PortProbe automatically sorts and deduplicates the list to `[8080, 8081, 8082]`.

### Startup Log Output:
```text
2026-11-20T18:15:00+05:30  INFO Listening on 0.0.0.0 across 8 ports: [80, 443, 8080, 8081, 8082, 8083, 8084, 8085]
```

---

## 3. Dynamic Configuration Reload (SIGHUP)

On Linux/Unix systems, PortProbe supports live configuration reloading without dropping the running process or disconnecting active TCP sessions.

### Triggering SIGHUP:
```bash
# By PID
kill -HUP <PID>

# Under systemd
sudo systemctl reload portprobe
```

### Dynamically Reloadable Parameters:
1. **CIDR Subnet Rules** (`allow_cidr`): When updated in `portprobe.toml`, SIGHUP parses and hot-reloads the new allowlist into the active `RwLock`. New connections immediately adhere to the updated subnet rules.
2. **TLS Server Certificate & Key** (`tls_cert`, `tls_key`): When certificates are renewed (e.g., via Let's Encrypt or corporate PKI), SIGHUP reloads the certificates, rebuilds the `TlsAcceptor`, and updates the certificate expiration gauge without dropping the listener.

### Reload Log Output:
```text
2026-11-20T18:15:30+05:30  INFO SIGHUP signal received; triggering dynamic configuration reload...
2026-11-20T18:15:30+05:30  INFO SIGHUP: Successfully reloaded 3 CIDR subnet rules from /etc/portprobe/portprobe.toml
2026-11-20T18:15:30+05:30  INFO SIGHUP: Successfully reloaded TLS certificates and private key from /etc/portprobe/certs/server.crt
```

---

## 4. Privilege Dropping (POSIX / Linux)

Running network tools as `root` is a major violation of security compliance standards (CIS Benchmarks, NIST SP 800-53). However, binding privileged ports (ports `< 1024` like 80 or 443) requires root privileges at startup.

PortProbe solves this using post-socket-bind privilege dropping:

```bash
sudo portprobe \
  --ports "80,443" \
  --user portprobe \
  --group portprobe
```

### How It Works:
1. PortProbe starts with root privileges (EUID 0) and binds ports 80 and 443.
2. Before accepting any client connections, PortProbe executes `drop_privileges()`, which permanently transitions the process UID and GID to the unprivileged `portprobe` account.
3. If running as root and privilege dropping fails, the daemon **strictly aborts** to prevent executing unprivileged workloads with elevated privileges.
4. If already running as an unprivileged user, the privilege drop is safely skipped with an informational log.

---

## 5. Graceful Daemon Shutdown

To stop PortProbe safely without terminating in-flight connections:
- Send `SIGINT` (`Ctrl+C`) or `SIGTERM`.

### Shutdown Sequence:
1. Listening sockets stop accepting new incoming connections.
2. The internal shutdown watch channel broadcasts `true` to all active worker tasks.
3. Active worker tasks complete their current response transmission.
4. The metrics summary table is emitted to the logger.
5. PortProbe exits with `ExitCode::SUCCESS` (0).
