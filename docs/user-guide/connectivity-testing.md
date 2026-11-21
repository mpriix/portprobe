# Connectivity Testing & Client Interoperability

**Document Reference:** `PORTPROBE-DOC-UG-04`  
**Document Version:** `2.0.0`  
**Classification:** Comprehensive User Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Interoperability Overview

While PortProbe includes its own native client probe (`--health-check`), enterprise infrastructure teams often use standard network utilities already present in their operating systems. PortProbe was designed for seamless interoperability with:
- Netcat (`nc`)
- Windows PowerShell (`Test-NetConnection`)
- Telnet
- cURL
- Enterprise Load Balancers (F5, AWS ALB/NLB, HAProxy)

---

## 2. Testing with Netcat (`nc`)

Netcat is standard across Linux, macOS, and Unix distributions.

### Basic Banner Verification:
```bash
nc 10.0.0.50 8080
```
Expected Output:
```text
PORTPROBE: TCP CONNECTION SUCCESSFUL
```

### Silent Port Probing (Check only):
```bash
nc -z 10.0.0.50 8080
echo $?
# Outputs 0 if reachable
```

---

## 3. Testing with Windows PowerShell

On Windows Server or Windows desktop clients:

### Quick TCP Port Check:
```powershell
Test-NetConnection -ComputerName 10.0.0.50 -Port 8080
```
Expected Output:
```text
ComputerName     : 10.0.0.50
RemoteAddress    : 10.0.0.50
RemotePort       : 8080
InterfaceAlias   : Ethernet0
SourceAddress    : 10.0.0.100
TcpTestSucceeded : True
```

### Raw Socket Banner Read in PowerShell:
To read the ASCII banner in PowerShell:
```powershell
$client = New-Object System.Net.Sockets.TcpClient("10.0.0.50", 8080)
$stream = $client.GetStream()
$reader = New-Object System.IO.StreamReader($stream)
$response = $reader.ReadLine()
Write-Host "Response from server: $response"
$client.Close()
```
Output:
```text
Response from server: PORTPROBE: TCP CONNECTION SUCCESSFUL
```

---

## 4. Testing with cURL & Load Balancers

### Testing Administrative Endpoints via cURL:

#### Check Health:
```bash
curl -i http://127.0.0.1:9090/healthz
```
Output:
```http
HTTP/1.1 200 OK
Content-Type: text/plain
Content-Length: 3
Connection: close

OK
```

#### Scrape Prometheus Metrics (With Bearer Token):
```bash
curl -i -H "Authorization: Bearer SecretToken123" http://127.0.0.1:9090/metrics
```
Output:
```http
HTTP/1.1 200 OK
Content-Type: text/plain; version=0.0.4; charset=utf-8
Content-Length: 520
Connection: close

# HELP portprobe_accepted_total Total accepted connections
# TYPE portprobe_accepted_total counter
portprobe_accepted_total 42
...
```

### Load Balancer Probes (AWS NLB, F5 BIG-IP):
- **TCP Three-Way Handshake Probes**: Configure the load balancer to send TCP SYN to the PortProbe listening port. PortProbe responds with SYN-ACK, completing the handshake.
- **HTTP Health Checks**: Point the load balancer's HTTP health monitor to `http://<host>:9090/healthz`. Supports both `GET` and `HEAD` requests.
