# PortProbe Quick-Start Guide

**Document Reference:** `PORTPROBE-DOC-GS-03`  
**Document Version:** `2.0.0`  
**Classification:** Getting Started Guide  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Quick-Start Objective

In this 5-minute quick start guide, you will:
1. Start your first PortProbe daemon on localhost.
2. Verify TCP reachability using PortProbe's native `--health-check` client probe.
3. Verify connectivity using standard network tools (Netcat and PowerShell).
4. Inspect connection events in human-readable and structured JSON logs.
5. Stop the daemon gracefully.

---

## 2. Step 1: Start the PortProbe Daemon

Open your terminal and run PortProbe on port `8080`:

```bash
portprobe --bind 127.0.0.1 --port 8080
```

### Expected Terminal Output:
```text
2026-11-20T18:10:00+05:30  INFO ====================================
2026-11-20T18:10:00+05:30  INFO           PORTPROBE STARTED
2026-11-20T18:10:00+05:30  INFO ====================================
2026-11-20T18:10:00+05:30  INFO Listening on 127.0.0.1:8080
2026-11-20T18:10:00+05:30  INFO Connection timeout: 15 seconds
2026-11-20T18:10:00+05:30  INFO Maximum connections: 100
2026-11-20T18:10:00+05:30  INFO Maximum connections per IP: 10
2026-11-20T18:10:00+05:30  INFO Maximum global rate: 20/second
2026-11-20T18:10:00+05:30  INFO Maximum per-IP rate: 5/second (SEC-FIND-16)
2026-11-20T18:10:00+05:30  INFO Silent rejection: true
2026-11-20T18:10:00+05:30  INFO Hold open mode: false
2026-11-20T18:10:00+05:30  INFO Waiting for incoming connections...
2026-11-20T18:10:00+05:30  INFO Press CTRL+C to stop
2026-11-20T18:10:00+05:30  INFO ====================================
```

> [!NOTE]  
> PortProbe defaults to secure enterprise defaults: `bind = 127.0.0.1`, `silent_reject = true`, and `hold_open = false` (anti-Slowloris immediate slot reclamation).

---

## 3. Step 2: Verify Reachability with Client Probe Mode

In a second terminal window, run PortProbe with the `--health-check` flag pointing to `127.0.0.1:8080`:

```bash
portprobe --health-check --bind 127.0.0.1 --port 8080
```

### Expected Output:
```text
PortProbe healthcheck SUCCESS: 127.0.0.1:8080 is reachable
```

Check the process exit code:
```bash
# Linux / macOS
echo $?
# Output: 0

# Windows (PowerShell)
$LASTEXITCODE
# Output: 0
```

---

## 4. Step 3: Verify Using External Client Tools

### Using Netcat (Linux / macOS):
```bash
nc 127.0.0.1 8080
```
Output:
```text
PORTPROBE: TCP CONNECTION SUCCESSFUL
```
*(The connection closes immediately post-banner due to anti-Slowloris default reclamation).*

### Using PowerShell (Windows):
```powershell
Test-NetConnection -ComputerName 127.0.0.1 -Port 8080
```
Output:
```text
ComputerName     : 127.0.0.1
RemoteAddress    : 127.0.0.1
RemotePort       : 8080
InterfaceAlias   : Loopback Pseudo-Interface 1
SourceAddress    : 127.0.0.1
TcpTestSucceeded : True
```

---

## 5. Step 4: Inspect Server Daemon Logs

In the first terminal window, observe how the daemon logged the connection lifecycle in real time:

```text
2026-11-20T18:10:15+05:30  INFO Connection accepted from 127.0.0.1:54320
2026-11-20T18:10:15+05:30  INFO Success response sent to 127.0.0.1:54320
```

---

## 6. Step 5: Graceful Shutdown

In the daemon terminal, press `Ctrl+C`.

### Expected Shutdown Output:
```text
2026-11-20T18:10:30+05:30  WARN Shutdown signal (Ctrl+C) received.
2026-11-20T18:10:30+05:30  INFO Stopping listeners...
2026-11-20T18:10:30+05:30  INFO Waiting for active connections to close...
2026-11-20T18:10:30+05:30  INFO All active connections closed.
2026-11-20T18:10:30+05:30  INFO ====================================
2026-11-20T18:10:30+05:30  INFO         CONNECTION METRICS
2026-11-20T18:10:30+05:30  INFO ====================================
2026-11-20T18:10:30+05:30  INFO Total accepted:       3
2026-11-20T18:10:30+05:30  INFO Rate limited:         0
2026-11-20T18:10:30+05:30  INFO Per-IP rate limited:  0
2026-11-20T18:10:30+05:30  INFO Connection limited:   0
2026-11-20T18:10:30+05:30  INFO Per-IP limited:       0
2026-11-20T18:10:30+05:30  INFO CIDR rejected:        0
2026-11-20T18:10:30+05:30  INFO Completed normally:   3
2026-11-20T18:10:30+05:30  INFO Timed out (greeting): 0
2026-11-20T18:10:30+05:30  INFO Errors:               0
2026-11-20T18:10:30+05:30  INFO ====================================
2026-11-20T18:10:30+05:30  INFO PortProbe stopped successfully.
```

Congratulations! You have successfully installed, executed, and verified PortProbe. Proceed to the [User Guide](../user-guide/overview.md) to explore multi-port binding, CIDR filtering, TLS encryption, and Prometheus metric integration.
