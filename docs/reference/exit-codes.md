# PortProbe Exit Codes Reference

**Document Reference:** `PORTPROBE-DOC-REF-04`  
**Document Version:** `2.0.0`  
**Classification:** Reference Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Process Exit Code Mapping

PortProbe returns standard POSIX and Windows process exit codes to allow seamless integration into automated bash scripts, CI/CD pipelines, and process managers (systemd, Docker):

| Exit Code | Constant | Meaning | Scenarios |
| :--- | :--- | :--- | :--- |
| `0` | `ExitCode::SUCCESS` | **Success** | - Healthcheck probe successfully verified reachability.<br/>- Daemon shut down cleanly following `SIGINT` (`Ctrl+C`) or `SIGTERM`.<br/>- Help (`--help`) or version (`--version`) displayed. |
| `1` | `ExitCode::FAILURE` | **Operational Failure** | - CLI argument validation failed (invalid port, invalid CIDR, out-of-range timeout).<br/>- Configuration file could not be read or contained invalid TOML.<br/>- Insecure permissions on key/token file (`mode & 0o077 != 0`).<br/>- Healthcheck probe failed (connection refused, timeout, non-success banner received).<br/>- Socket bind failed (address already in use, permission denied).<br/>- Privilege dropping failed while running as root (EUID 0).<br/>- TLS certificates could not be loaded or parsed. |

---

## 2. Using Exit Codes in Automation

### Bash Pipeline Evaluation:
```bash
portprobe --health-check --bind 10.0.0.50 --port 8080
STATUS=$?

if [ $STATUS -eq 0 ]; then
    echo "PortProbe check succeeded. Starting application deployment."
else
    echo "PortProbe check failed with exit code $STATUS. Aborting." >&2
    exit 1
fi
```

### PowerShell Pipeline Evaluation:
```powershell
& portprobe.exe --health-check --bind 10.0.0.50 --port 8080
if ($LASTEXITCODE -eq 0) {
    Write-Host "Connectivity verified."
} else {
    Write-Error "Connectivity verification failed with exit code $LASTEXITCODE."
    exit 1
}
```
