# PortProbe Deployment Architecture Guide

**Document Reference:** `PORTPROBE-DOC-DEP-01`  
**Document Version:** `2.0.0`  
**Classification:** Enterprise Deployment Specification  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Enterprise Deployment Topologies

PortProbe can be deployed in three standard enterprise topologies depending on the verification goal:

### Topology 1: On-Demand Diagnostic Probe (Ephemeral)
Used during active firewall cutovers or infrastructure provisioning. An engineer starts PortProbe interactively via CLI or an automation script (Ansible, Terraform), conducts reachability tests, and immediately tears down the listener.

### Topology 2: Long-Running Dedicated Connectivity Canary (systemd)
Deployed as a persistent background daemon on designated infrastructure canary hosts. Continually verifies cross-VPC, hybrid-cloud, and multi-region routing paths, while emitting Prometheus metrics to Grafana dashboards.

### Topology 3: Containerized Microservice Pod (Kubernetes / OCI)
Packaged as a lightweight container deployed into Kubernetes clusters or container hosts to validate ingress controllers, network policies (Calico, Cilium), and service mesh egress gateways.

```mermaid
flowchart TD
    subgraph Multi-Tier Enterprise VPC
        LB[External Load Balancer]
        DMZ[DMZ Web Proxy Subnet]
        APP[App Subnet]
        DB[Database Subnet]
    end

    subgraph PortProbe Canary Instances
        PP_DMZ[PortProbe Daemon :80, :443<br/>Node: dmz-canary-01]
        PP_APP[PortProbe Daemon :8080-8085<br/>Node: app-canary-01]
        PP_DB[PortProbe Daemon :1521, :5432<br/>Node: db-canary-01]
    end

    LB -->|Health Check| PP_DMZ
    PP_DMZ -->|Verify DMZ-to-App ACL| PP_APP
    PP_APP -->|Verify App-to-DB ACL| PP_DB
    
    subgraph Observability
        PROM[Prometheus Scraper]
    end
    
    PROM -.->|Scrape :9090/metrics| PP_DMZ
    PROM -.->|Scrape :9090/metrics| PP_APP
    PROM -.->|Scrape :9090/metrics| PP_DB
```

---

## 2. High Availability & Redundancy

Because PortProbe is an authoritative connectivity probe rather than a stateful database or transactional server:
- Deploy independent PortProbe canary instances across each Availability Zone (AZ) or redundant data center rack.
- Front multiple PortProbe instances with a network load balancer (NLB) to test both physical path reachability and load-balancing distribution algorithms simultaneously.
- Configuration is stateless; instances do not share session state, enabling linear scaling.
