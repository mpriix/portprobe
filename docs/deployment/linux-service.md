# Linux Service Deployment Guide (`systemd`)

**Document Reference:** `PORTPROBE-DOC-DEP-02`  
**Document Version:** `2.0.0`  
**Classification:** Enterprise Deployment Specification  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Production `systemd` Configuration

PortProbe includes a fully hardened, production-ready systemd unit file (`portprobe.service`) adhering to CIS Linux Benchmarks and NIST SP 800-53 security controls.

### Service Unit File (`/etc/systemd/system/portprobe.service`):
```ini
[Unit]
Description=PortProbe Secure TCP Connectivity Testing Service
Documentation=https://github.com/mpriix/portprobe
After=network.target

[Service]
Type=simple
User=portprobe
Group=portprobe
ExecStart=/usr/local/bin/portprobe --config /etc/portprobe/portprobe.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s

# Security Hardening Directives (NIST SP 800-53 / CIS Linux Benchmark)
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_BIND_SERVICE
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
PrivateTmp=true
PrivateDevices=true
MemoryDenyWriteExecute=true
LockPersonality=true
RestrictRealtime=true
RestrictSUIDSGID=true
RestrictAddressFamilies=AF_INET AF_INET6

# Resource and File Descriptor Limits
LimitNOFILE=4096

[Install]
WantedBy=multi-user.target
```

---

## 2. Step-by-Step Installation & Management

### Step 1: Create Dedicated Service Account
```bash
sudo useradd --system --no-create-home --user-group --shell /sbin/nologin portprobe
```

### Step 2: Install Binary and Service Unit
```bash
sudo cp target/release/portprobe /usr/local/bin/portprobe
sudo chmod 0755 /usr/local/bin/portprobe
sudo cp portprobe.service /etc/systemd/system/portprobe.service
```

### Step 3: Configure Permissions
```bash
sudo mkdir -p /etc/portprobe
sudo cp portprobe.toml /etc/portprobe/portprobe.toml
sudo chown -R root:portprobe /etc/portprobe
sudo chmod 0750 /etc/portprobe
sudo chmod 0640 /etc/portprobe/portprobe.toml
```

### Step 4: Reload, Enable, and Start
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now portprobe
```

### Step 5: Verify Service Status and Logs
```bash
# Check service status
sudo systemctl status portprobe

# View live structured logs
sudo journalctl -u portprobe -f
```

### Step 6: Live Configuration Reload
To update CIDR allowlists or TLS certificates without stopping the service:
```bash
sudo systemctl reload portprobe
```
*(Executes `/bin/kill -HUP $MAINPID`, triggering PortProbe's dynamic reload handler).*
