# PortProbe Enterprise Product Roadmap

**Document Reference:** `PORTPROBE-DOC-REL-03`  
**Document Version:** `2.0.0`  
**Classification:** Enterprise Strategic Roadmap  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Roadmap Evolution & Planned Capabilities

The PortProbe product roadmap outlines planned enhancements across upcoming minor and major releases:

```mermaid
flowchart LR
    V20["Version 2.0.0<br/>(Current Stable)<br/>Multi-Port, TLS 1.3, mTLS<br/>Admin HTTP, JSON Logging<br/>Admission Controls"]
    
    V21["Version 2.1.0<br/>(Target: Q1 2027)<br/>Native Windows SCM Dispatcher<br/>TLS on Admin Endpoint<br/>NTFS DACL Validation"]
    
    V22["Version 2.2.0<br/>(Target: Q2 2027)<br/>UDP Connectivity Probing<br/>DNS SRV/A Health Checks<br/>OpenTelemetry Tracing"]
    
    V30["Version 3.0.0<br/>(Target: Q4 2027)<br/>Distributed Probe Mesh<br/>Prometheus Remote-Write<br/>Kubernetes Operator"]

    V20 --> V21
    V21 --> V22
    V22 --> V30
```

---

## 2. Release Milestones & Feature Breakdown

### Version 2.1.0 (Q1 2027)
- **Native Windows Service Dispatcher**: Integrate `windows-service` crate to link `--service` directly to the Windows Service Control Manager (SCM).
- **TLS on Administrative Endpoint**: Add `--admin-tls` to allow secure remote Prometheus scraping over HTTPS.
- **Windows NTFS DACL Validation**: Native Win32 API checks verifying that secret token files are accessible only by the service owner.

### Version 2.2.0 (Q2 2027)
- **UDP Connectivity Verification**: Support UDP echo/diagnostic listeners and client probing.
- **DNS Resolution Probing**: Active validation of split-horizon DNS, SRV records, and reverse lookups.
- **OpenTelemetry (OTel) Native Tracing**: Export distributed trace spans for connection lifecycle diagnostics.

### Version 3.0.0 (Q4 2027)
- **PortProbe Mesh Coordinator**: Centralized coordination daemon aggregating distributed probe results across multi-cloud VPCs.
- **Kubernetes Custom Resource Definition (CRD) & Operator**: Automate canary probe deployment across Kubernetes clusters.
