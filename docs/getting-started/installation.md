# PortProbe Installation Guide

**Document Reference:** `PORTPROBE-DOC-GS-02`  
**Document Version:** `2.0.0`  
**Classification:** Getting Started Guide  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Installation Overview

PortProbe can be installed using four primary methods depending on your infrastructure architecture:
1. **Building from source with Cargo** (Universal Linux, macOS, Windows).
2. **Manual binary deployment** (Enterprise Linux servers).
3. **Containerized deployment** (Docker, Podman, Kubernetes).
4. **systemd service installation** (Production Linux servers).

---

## 2. Building from Source

### Step 1: Clone the Repository
```bash
git clone https://github.com/mpriix/portprobe.git
cd portprobe
```

### Step 2: Compile the Release Binary
Execute Cargo in release mode:
```bash
cargo build --release
```

The optimized binary with thin LTO (Link-Time Optimization) and stripped debug symbols will be produced at:
- **Linux / macOS**: `target/release/portprobe`
- **Windows**: `target\release\portprobe.exe`

### Step 3: Verify Binary Build
```bash
./target/release/portprobe --version
```
Expected output:
```text
portprobe 2.0.0
```

---

## 3. Linux System-Wide Binary Installation

To install the PortProbe binary into the standard system path:

```bash
# 1. Copy binary to /usr/local/bin
sudo cp target/release/portprobe /usr/local/bin/portprobe
sudo chmod 0755 /usr/local/bin/portprobe

# 2. Verify global execution
portprobe --help
```

### Optional: Grant Privileged Port Binding Capability
If you plan to run PortProbe on privileged ports (ports < 1024 such as 80 or 443) without executing the entire daemon as root:
```bash
sudo setcap 'cap_net_bind_service=+ep' /usr/local/bin/portprobe
```

---

## 4. Container Deployment (Docker & Podman)

PortProbe includes a production-ready, multi-stage, hardened `Dockerfile` based on Alpine Linux 3.20. It executes as an unprivileged service user (`portprobe`, UID 10001) and includes a native health check.

### Step 1: Build Container Image
```bash
docker build -t portprobe:2.0.0 .
```

### Step 2: Run Container
To run a container listening on port 8080:
```bash
docker run -d \
  --name portprobe \
  --restart unless-stopped \
  -p 8080:8080 \
  -e PORTPROBE_BIND=0.0.0.0 \
  -e PORTPROBE_PORT=8080 \
  portprobe:2.0.0
```

### Step 3: Inspect Container Health Status
```bash
docker inspect --format='{{json .State.Health.Status}}' portprobe
```
Output:
```text
"healthy"
```

---

## 5. Linux systemd Service Installation

For long-running background daemon operation on Linux servers (RHEL, Rocky, Ubuntu, Debian):

### Step 1: Create Dedicated Service User
```bash
sudo useradd --system --no-create-home --user-group --shell /sbin/nologin portprobe
```

### Step 2: Provision Configuration Directory
```bash
sudo mkdir -p /etc/portprobe
sudo cp portprobe.toml /etc/portprobe/portprobe.toml
sudo chown -R root:portprobe /etc/portprobe
sudo chmod 0750 /etc/portprobe
sudo chmod 0640 /etc/portprobe/portprobe.toml
```

### Step 3: Install systemd Unit File
Copy the included `portprobe.service` file to systemd:
```bash
sudo cp portprobe.service /etc/systemd/system/portprobe.service
sudo systemctl daemon-reload
```

### Step 4: Enable and Start Service
```bash
sudo systemctl enable --now portprobe
sudo systemctl status portprobe
```

---

## 6. Windows Installation

### Step 1: Binary Placement
1. Create a dedicated directory: `C:\Program Files\PortProbe`
2. Copy `portprobe.exe` to `C:\Program Files\PortProbe\portprobe.exe`
3. Copy `portprobe.toml` to `C:\Program Files\PortProbe\portprobe.toml`

### Step 2: Add to System PATH
In an administrative PowerShell prompt:
```powershell
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::Machine) + ";C:\Program Files\PortProbe",
    [EnvironmentVariableTarget]::Machine
)
```

### Step 3: Windows Background Service Setup (via NSSM)
> [!NOTE]  
> While PortProbe exposes a `--service` flag on the CLI, full Windows Service Control Manager (SCM) dispatching is currently a roadmap feature (see Release Notes). To run PortProbe as a resilient Windows service today, use the Non-Sucking Service Manager (NSSM):

```powershell
# Install service using NSSM
nssm install PortProbe "C:\Program Files\PortProbe\portprobe.exe" "--config" "C:\Program Files\PortProbe\portprobe.toml"
nssm set PortProbe AppStdout "C:\Program Files\PortProbe\portprobe.log"
nssm set PortProbe AppStderr "C:\Program Files\PortProbe\portprobe_err.log"
nssm start PortProbe
```

---

## 7. Uninstallation

### Linux
```bash
sudo systemctl stop portprobe
sudo systemctl disable portprobe
sudo rm -f /etc/systemd/system/portprobe.service
sudo systemctl daemon-reload
sudo rm -f /usr/local/bin/portprobe
sudo rm -rf /etc/portprobe
sudo userdel portprobe
```

### Windows
```powershell
# If installed via NSSM
nssm stop PortProbe
nssm remove PortProbe confirm

# Remove files
Remove-Item -Recurse -Force "C:\Program Files\PortProbe"
```
