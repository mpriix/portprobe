# Production Best Practices & Security Hardening

**Document Reference:** `PORTPROBE-DOC-DEP-03`  
**Document Version:** `2.0.0`  
**Classification:** Enterprise Operations Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Production Security Checklist

Follow this checklist before running PortProbe in production environments:

- [ ] **Interface Binding**: Explicitly bind to private internal IP addresses (e.g., `--bind 10.200.1.50`). Never use `0.0.0.0` unless protected by perimeter firewalls.
- [ ] **CIDR Ingress Filtering**: Always define `--allow-cidr` to restrict incoming connections exclusively to authorized application or management subnets.
- [ ] **Silent Rejection Active**: Maintain `--silent-reject true` (the default) to prevent port scanners from detecting the PortProbe banner.
- [ ] **Immediate Slot Reclamation**: Keep `--hold-open false` (the default) unless conducting active, monitored idle-session testing.
- [ ] **Administrative Token File**: Protect `--admin-port` using an unguessable Bearer token loaded via `--admin-token-file`. Enforce `chmod 0600` on the token file.
- [ ] **File Descriptor Limits**: Set `LimitNOFILE=65535` if configuring `--max-connections > 500`.
- [ ] **Privilege Dropping**: If binding privileged ports (< 1024), configure `--user portprobe --group portprobe`.
- [ ] **Log Ingestion**: Configure `--log-format json` and ship logs to a centralized SIEM (Splunk, Elastic, Datadog).

---

## 2. Secrets Management & Rotation

### Admin Token Rotation:
1. Generate a new high-entropy token:
   ```bash
   openssl rand -hex 32 > /etc/portprobe/admin_token.secret.new
   chmod 0600 /etc/portprobe/admin_token.secret.new
   chown portprobe:portprobe /etc/portprobe/admin_token.secret.new
   mv /etc/portprobe/admin_token.secret.new /etc/portprobe/admin_token.secret
   ```
2. Restart the daemon or reload if integrated into future reloadable paths.

### TLS Certificate Rotation:
1. Replace `/etc/portprobe/certs/server.crt` and `/etc/portprobe/certs/server.key` with the newly signed certificates.
2. Ensure `chmod 0600 /etc/portprobe/certs/server.key`.
3. Send `SIGHUP` to the running daemon:
   ```bash
   sudo systemctl reload portprobe
   ```
4. Verify Prometheus metric `portprobe_tls_cert_expiry_timestamp_seconds` reflects the updated expiration date.
