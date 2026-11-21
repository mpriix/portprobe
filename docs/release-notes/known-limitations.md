# PortProbe Known Limitations & Operational Boundaries

**Document Reference:** `PORTPROBE-DOC-REL-02`  
**Document Version:** `2.0.0`  
**Classification:** Release Specification  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Verified Implementation Gaps & Limitations

To ensure engineering transparency and operational safety, the following known limitations in PortProbe 2.0.0 are documented along with their operational mitigations:

### 1.1 Windows Service Flag (`--service`)
- **Limitation**: The `--service` flag is declared in `src/cli.rs:253`, but `src/main.rs` does not currently invoke the Windows Service Control Manager (SCM) dispatcher. Running `portprobe --service` in Windows PowerShell starts in standard console mode.
- **Operational Workaround**: To deploy PortProbe as a resilient background service on Windows Server, install it using NSSM (Non-Sucking Service Manager) or Windows Task Scheduler at system boot. See [docs/getting-started/installation.md](../getting-started/installation.md).

### 1.2 Administrative Server TLS
- **Limitation**: The administrative HTTP endpoint (`--admin-port`) operates over plain HTTP on `127.0.0.1`. It does not support TLS encryption directly.
- **Operational Workaround**: PortProbe strictly binds the administrative server to `127.0.0.1` (localhost), ensuring traffic does not leave the local loopback interface. If remote Prometheus scraping is required, route requests through a reverse proxy (e.g., Nginx, Envoy) or SSH tunnel.

### 1.3 Transport Protocol Scope (No UDP Support)
- **Limitation**: PortProbe 2.0.0 exclusively verifies TCP and TLS-over-TCP reachability. It does not support UDP port reachability testing (e.g., DNS port 53, SNMP port 161).
- **Operational Workaround**: For UDP reachability verification, continue using traditional tools such as `nc -u` or specialized UDP probes until UDP support is released in PortProbe 2.2.0.

### 1.4 Windows File Permission Validation
- **Limitation**: The secret file permission check (`verify_file_permissions` in `src/cli.rs`) validates POSIX octal file modes (`0600`/`0400`). On Windows, this check is a no-op because Windows uses NTFS Discretionary Access Control Lists (DACLs).
- **Operational Workaround**: Administrators on Windows Server must ensure that secret token files and TLS private keys have restricted NTFS permissions via `icacls`, granting read access exclusively to the service account or `SYSTEM`.

### 1.5 Dynamic SIGHUP Reload on Windows
- **Limitation**: Operating system signals like `SIGHUP` are POSIX-specific and not supported by Windows kernels.
- **Operational Workaround**: On Windows Server, configuration updates require a service restart via `Restart-Service PortProbe` or NSSM.
