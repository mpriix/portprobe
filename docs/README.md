# PortProbe Enterprise Documentation Portal

**Product Version:** `2.0.0`  
**Document Classification:** Enterprise Software Documentation  
**Documentation Status:** Official Release  
**Last Updated:** November 2026  

Welcome to the official documentation portal for **PortProbe**, the enterprise-grade asynchronous TCP connectivity verification daemon and client probe written in Rust.

---

## Navigation Directory

The PortProbe documentation package is organized into dedicated topic sections:

```
docs/
├── README.md                              <- Portal home (this document)
├── DOCUMENTATION_AUDIT_REPORT.md          <- Comprehensive documentation verification & audit report
│
├── product/                               <- Product overview, vision, and technical design
│   ├── product-overview.md                <- Problem statement, target audience, and capabilities
│   ├── architecture.md                    <- System architecture, component models, and concurrency
│   └── technical-design.md                <- Deep-dive technical implementation & internal state
│
├── getting-started/                       <- Installation and initial setup
│   ├── prerequisites.md                   <- System, network, and compiler requirements
│   ├── installation.md                    <- Build from source, Linux, Windows, macOS, Docker
│   └── quick-start.md                     <- First connectivity verification and health check
│
├── user-guide/                            <- Comprehensive operational user guide
│   ├── overview.md                        <- User guide roadmap and feature overview
│   ├── daemon-management.md               <- Daemon lifecycle, multi-port binding, signals
│   ├── client-usage.md                    <- Client probe mode (--health-check, TLS verification)
│   ├── connectivity-testing.md            <- Interfacing with Netcat, Telnet, PowerShell, curl
│   ├── concurrency.md                     <- Rate limiters, token buckets, and connection caps
│   ├── configuration.md                   <- TOML configuration files, CLI precedence, environment
│   └── practical-examples.md              <- Realistic deployment invocations and commands
│
├── reference/                             <- Authoritative reference manuals
│   ├── cli-reference.md                   <- Exhaustive CLI flags, options, syntax, and defaults
│   ├── configuration-reference.md         <- Parameter-by-parameter TOML reference
│   ├── error-reference.md                 <- Daemon errors, banner codes, and HTTP responses
│   └── exit-codes.md                      <- CLI process exit codes and error mapping
│
├── deployment/                            <- Production operations and infrastructure deployment
│   ├── deployment-guide.md                <- Enterprise deployment topology and architecture
│   ├── linux-service.md                   <- systemd unit setup, SIGHUP reload, privilege dropping
│   ├── production-best-practices.md       <- CIS benchmarks, secret isolation, network hardening
│   └── performance-tuning.md              <- OS file descriptors (ulimit), backlog, kernel tuning
│
├── troubleshooting/                       <- Diagnostics, remediation, and operational FAQ
│   ├── troubleshooting-guide.md           <- Systematic issue resolution and log analysis
│   └── faq.md                             <- Frequently Asked Questions across all domains
│
├── security/                              <- Security architecture and threat modeling
│   └── security-guide.md                  <- NIST SP 800-53 / CIS controls, mTLS, admission control
│
├── development/                           <- Contributor, architecture, and developer resources
│   ├── developer-guide.md                 <- Source organization, modules, and coding standards
│   ├── build-and-test.md                  <- Test execution, Clippy, cargo-audit, and CI gates
│   └── contributing.md                    <- Contribution guidelines, security bug disclosure
│
├── examples/                              <- Real-world enterprise scenarios
│   ├── firewall-validation.md             <- Scenario 1: Firewall rule verification & DMZ testing
│   ├── load-balancer-validation.md        <- Scenario 2: Load balancer health probe configuration
│   ├── infrastructure-validation.md       <- Scenario 3: Post-provisioning infrastructure validation
│   └── ci-cd-integration.md               <- Scenario 4: Automated CI/CD deployment gates
│
└── release-notes/                         <- Release engineering, limitations, and future plans
    ├── changelog.md                       <- Version history and detailed changelog
    ├── known-limitations.md               <- Verified product gaps and operational workarounds
    └── roadmap.md                         <- Future roadmap and planned enterprise capabilities
```

---

## Documentation Conventions

Throughout this documentation, the following formatting conventions are used:

> [!NOTE]  
> Highlights background context, operational recommendations, or important behavioral characteristics.

> [!TIP]  
> Suggests performance optimizations, security hardening best practices, or command-line shortcuts.

> [!IMPORTANT]  
> Emphasizes critical operational steps, mandatory configuration parameters, or security invariants.

> [!WARNING]  
> Identifies dangerous configurations, potential operational pitfalls, or security risks.

> [!CAUTION]  
> Alerts to high-risk actions that could result in service disruption, denial of service, or security exposure.

---

## Verifiability and Source Code Fidelity

Every command, configuration key, flag, and code snippet in this documentation has been verified against the PortProbe version 2.0.0 codebase (`src/main.rs`, `src/cli.rs`, `src/server.rs`, `src/logger.rs`). 

Features marked as **Implemented** reflect fully functional code supported by automated test suites. Features marked as **Operational Workaround** or **Planned Roadmap** represent known implementation boundaries (such as Windows Service integration or UDP prober support) and are explicitly identified to maintain strict engineering integrity.
