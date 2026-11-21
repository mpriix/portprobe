# Real-World Practical Examples & Recipes

**Document Reference:** `PORTPROBE-DOC-UG-07`  
**Document Version:** `2.0.0`  
**Classification:** Comprehensive User Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Practical Recipe Collection

Below are tested, ready-to-use command recipes for real-world enterprise infrastructure tasks.

---

### Recipe 1: Database Firewall Port Validation (Oracle TCP 1521)
**Scenario**: You requested the firewall team to open port 1521 between an App server (`10.10.10.20`) and a DB server (`10.20.20.50`). The Oracle database is not yet installed.

**On DB Server (`10.20.20.50`):**
```bash
portprobe \
  --bind 10.20.20.50 \
  --port 1521 \
  --allow-cidr "10.10.10.20/32" \
  --log-format json
```

**On App Server (`10.10.10.20`):**
```bash
portprobe --health-check --bind 10.20.20.50 --port 1521
```
Expected Output:
```text
PortProbe healthcheck SUCCESS: 10.20.20.50:1521 is reachable
```

---

### Recipe 2: Multi-Port Web Proxy Verification (Ports 80, 443, 8080–8085)
**Scenario**: A DMZ reverse proxy needs to listen across multiple HTTP and backend microservice ports.

```bash
portprobe \
  --bind 0.0.0.0 \
  --ports "80,443,8080-8085" \
  --max-connections 2000 \
  --max-rate 200 \
  --admin-port 9090 \
  --log-format json
```

**Verifying all ports in a bash loop:**
```bash
for port in 80 443 8080 8081 8082 8083 8084 8085; do
    portprobe --health-check --bind 127.0.0.1 --port "$port"
done
```

---

### Recipe 3: Strict Banking TLS 1.3-Only Encrypted Listener
**Scenario**: Banking regulatory compliance requires all network traffic to be encrypted using TLS 1.3 exclusively with mTLS client certificate validation.

**Generate test certificates:**
```bash
openssl req -x509 -newkey rsa:4096 -keyout server.key -out server.crt -days 365 -nodes -subj "/CN=db.example.com"
chmod 0600 server.key
```

**Start PortProbe:**
```bash
portprobe \
  --bind 0.0.0.0 \
  --port 8443 \
  --tls-cert server.crt \
  --tls-key server.key \
  --tls-min-version "1.3"
```

**Verify with TLS client probe:**
```bash
portprobe \
  --health-check \
  --bind 127.0.0.1 \
  --port 8443 \
  --health-check-tls \
  --health-check-domain "db.example.com" \
  --health-check-ca server.crt
```

---

### Recipe 4: Stress-Testing Load Balancer Concurrency Limits
**Scenario**: Verify that an upstream load balancer correctly handles rate limiting when downstream traffic spikes.

```bash
portprobe \
  --bind 0.0.0.0 \
  --port 8080 \
  --max-connections 50 \
  --max-connections-per-ip 5 \
  --max-rate 10 \
  --max-rate-per-ip 2 \
  --silent-reject false
```
Clients exceeding 2 req/s will receive `PORTPROBE: RATE LIMIT EXCEEDED\n`.
