# PortProbe Automated Test Case Catalog & Execution Audit

**Document Reference:** `PORTPROBE-TEST-CATALOG-2026`  
**Application:** PortProbe v2.0.0  
**Total Automated Test Cases:** 139  
**Execution Pass Rate:** 100% (139 Passed, 0 Failed, 0 Ignored)  

---

## 1. Overview & Test Breakdown

The automated test catalog is organized into functional suites:

| Suite / Test File | Test Category | Total Tests | Passed | Failed | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`src/lib.rs` (Internal Unit Tests)** | Module & Logic Unit Testing | **84** | 84 | 0 | **PASSED** |
| **`tests/unit_tests.rs`** | Parameter & State Unit Testing | **13** | 13 | 0 | **PASSED** |
| **`tests/integration_tests.rs`** | Component Integration & Daemon Lifecycle | **4** | 4 | 0 | **PASSED** |
| **`tests/networking_tests.rs`** | TCP Reachability, Latency & TLS Scenarios | **9** | 9 | 0 | **PASSED** |
| **`tests/concurrency_tests.rs`** | High-Concurrency & Admission Control Caps | **4** | 4 | 0 | **PASSED** |
| **`tests/configuration_tests.rs`** | TOML Loading, Precedence & Error Rejection | **9** | 9 | 0 | **PASSED** |
| **`tests/cli_tests.rs`** | CLI Binary Execution, Flags & Exit Codes | **9** | 9 | 0 | **PASSED** |
| **`tests/reliability_tests.rs`** | Stability, Disconnects & Graceful Shutdown | **4** | 4 | 0 | **PASSED** |
| **`tests/security_tests.rs`** | Access Controls, Slowloris & Side-Channel | **3** | 3 | 0 | **PASSED** |
| **TOTAL** | **Full Regression & Verification Suite** | **139** | **139** | **0** | **100% PASS** |

---

## 2. Detailed Test Case Catalog

### 2.1 Internal Unit Tests (`src/lib.rs` - 84 Cases)

| Test ID | Module | Feature Tested | Objective / Test Steps | Input / Condition | Expected Result | Actual Result | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **TC-UNIT-01** | `cli.rs` | Timeout Lower Bound | Validate minimum valid timeout is accepted | `"1"` | `Ok(1)` | `Ok(1)` | **PASSED** |
| **TC-UNIT-02** | `cli.rs` | Timeout Typical Value | Validate standard timeout parsing | `"15"` | `Ok(15)` | `Ok(15)` | **PASSED** |
| **TC-UNIT-03** | `cli.rs` | Timeout Upper Bound | Validate maximum 1-hour timeout | `"3600"` | `Ok(3600)` | `Ok(3600)` | **PASSED** |
| **TC-UNIT-04** | `cli.rs` | Timeout Zero Value | Validate rejection of zero timeout | `"0"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-05** | `cli.rs` | Timeout Exceeds Upper Bound | Validate rejection of > 3600s | `"3601"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-06** | `cli.rs` | Timeout Non-Numeric | Validate rejection of alphabetic input | `"abc"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-07** | `cli.rs` | Timeout Negative | Validate rejection of negative number | `"-1"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-08** | `cli.rs` | Global Connections Lower Bound | Validate lower bound 1 | `"1"` | `Ok(1)` | `Ok(1)` | **PASSED** |
| **TC-UNIT-09** | `cli.rs` | Global Connections Standard | Validate standard 100 connections | `"100"` | `Ok(100)` | `Ok(100)` | **PASSED** |
| **TC-UNIT-10** | `cli.rs` | Global Connections Upper Bound | Validate upper bound 10,000 | `"10000"` | `Ok(10000)` | `Ok(10000)` | **PASSED** |
| **TC-UNIT-11** | `cli.rs` | Global Connections Zero | Validate rejection of 0 connections | `"0"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-12** | `cli.rs` | Global Connections Overflow | Validate rejection of > 10,000 | `"10001"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-13** | `cli.rs` | Per-IP Connections Lower Bound | Validate per-IP lower bound 1 | `"1"` | `Ok(1)` | `Ok(1)` | **PASSED** |
| **TC-UNIT-14** | `cli.rs` | Per-IP Connections Standard | Validate standard 10 per IP | `"10"` | `Ok(10)` | `Ok(10)` | **PASSED** |
| **TC-UNIT-15** | `cli.rs` | Per-IP Connections Upper Bound | Validate per-IP upper bound 1,000 | `"1000"` | `Ok(1000)` | `Ok(1000)` | **PASSED** |
| **TC-UNIT-16** | `cli.rs` | Per-IP Connections Zero | Validate rejection of 0 per IP | `"0"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-17** | `cli.rs` | Per-IP Connections Overflow | Validate rejection of > 1,000 | `"1001"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-18** | `cli.rs` | Global Rate Lower Bound | Validate rate lower bound 1 req/s | `"1"` | `Ok(1)` | `Ok(1)` | **PASSED** |
| **TC-UNIT-19** | `cli.rs` | Global Rate Standard | Validate rate standard 20 req/s | `"20"` | `Ok(20)` | `Ok(20)` | **PASSED** |
| **TC-UNIT-20** | `cli.rs` | Global Rate Upper Bound | Validate rate upper bound 1,000 | `"1000"` | `Ok(1000)` | `Ok(1000)` | **PASSED** |
| **TC-UNIT-21** | `cli.rs` | Global Rate Zero Value | Validate rejection of 0 rate | `"0"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-22** | `cli.rs` | Global Rate Overflow | Validate rejection of > 1,000 | `"1001"` | `Err` | `Err` (Rejected) | **PASSED** |
| **TC-UNIT-23** | `cli.rs` | Per-IP Rate Bounds (1..1000) | Validate valid per-IP rates and bounds | `"1"`, `"5"`, `"1000"` | `Ok` on valid, `Err` on 0/1001 | Matches expected | **PASSED** |
| **TC-UNIT-24** | `cli.rs` | Port Zero Rejection | Validate rejection of port 0 | `port = 0` | `validate()` returns `Err` | `Err` | **PASSED** |
| **TC-UNIT-25** | `cli.rs` | Limit Conflict (Per-IP > Global) | Rejects per-IP limit exceeding global cap | `conns = 10, per_ip = 50` | `validate()` returns `Err` | `Err` | **PASSED** |
| **TC-UNIT-26** | `cli.rs` | Limit Equality (Per-IP == Global)| Accepts equal per-IP and global caps | `conns = 100, per_ip = 100` | `validate()` returns `Ok` | `Ok(())` | **PASSED** |
| **TC-UNIT-27** | `cli.rs` | Rate Conflict (Per-IP > Global) | Rejects per-IP rate exceeding global rate | `rate = 10, per_ip_rate = 20` | `validate()` returns `Err` | `Err` | **PASSED** |
| **TC-UNIT-28** | `cli.rs` | CIDR IPv4 Range Matching | Tests IP containment in IPv4 subnet | `10.0.0.0/8` vs `10.0.0.1` | `contains == true` | `true` | **PASSED** |
| **TC-UNIT-29** | `cli.rs` | CIDR IPv4 Single Host | Tests host mask without prefix | `192.168.1.50` | Matches only .50 | Matches only .50 | **PASSED** |
| **TC-UNIT-30** | `cli.rs` | CIDR IPv6 Subnet Matching | Tests IPv6 prefix containment | `2001:db8::/32` | `contains == true` | `true` | **PASSED** |
| **TC-UNIT-31** | `cli.rs` | CIDR Whitespace Handling | Parses CIDR with surrounding whitespace | `"  10.0.0.0 / 8  "` | Parses cleanly | `Ok` | **PASSED** |
| **TC-UNIT-32** | `cli.rs` | CIDR Invalid Strings | Rejects bad prefixes and syntax | `"10.0.0.1/33"`, `"not-ip"` | `Err` | `Err` | **PASSED** |
| **TC-UNIT-33** | `cli.rs` | IPv4-Mapped IPv6 Dual-Stack | Normalizes IPv4-mapped IPv6 in CIDR | `::ffff:192.168.1.0/120` | Matches mapped IPv4 | `true` | **PASSED** |
| **TC-UNIT-34** | `cli.rs` | TLS Cert / Key Co-requirement | Rejects `--tls-cert` without `--tls-key` | `cert = Some, key = None` | `validate()` returns `Err` | `Err` | **PASSED** |
| **TC-UNIT-35** | `cli.rs` | TLS Min Version Allowed | Accepts `"1.2"` and `"1.3"`; rejects others | `"1.3"`, `"1.1"` | `Ok` on 1.3, `Err` on 1.1 | Matches expected | **PASSED** |
| **TC-UNIT-36** | `cli.rs` | Admin Token File Reading | Loads token from file with validation | Path to token file | `admin_token` populated | Matches expected | **PASSED** |
| **TC-UNIT-37** | `cli.rs` | Empty Admin Token Rejection | Rejects whitespace-only token | `"   "` | `validate()` returns `Err` | `Err` | **PASSED** |
| **TC-UNIT-38** | `cli.rs` | Port Range Expansion | Expands discrete + spans | `"80,443,8085-8088"` | `vec![80,443,8085..8088]` | Sorted list | **PASSED** |
| **TC-UNIT-39** | `cli.rs` | Port Deduplication | Removes duplicate ports across lists | `"8080", "8080-8082"` | `vec![8080, 8081, 8082]` | Deduplicated | **PASSED** |
| **TC-UNIT-40** | `cli.rs` | Total Ports Cap (50 Max) | Rejects > 50 total listening ports | `"1000-1060"` (61 ports) | `resolve_all_ports` Err | `Err` | **PASSED** |
| **TC-UNIT-41** | `cli.rs` | Config File Merging | Loads TOML settings into empty CLI | Valid TOML path | All fields populated | Fields populated | **PASSED** |
| **TC-UNIT-42** | `cli.rs` | CLI Precedence Over File | Explicit CLI args override TOML | CLI `--bind 127.0.0.1` | Preserves CLI value | Preserved | **PASSED** |
| **TC-UNIT-43** | `cli.rs` | Secure Default Resolution | Defaults assigned if unspecified | Unset parameters | Bind 127.0.0.1, silent true | Assigned | **PASSED** |
| **TC-UNIT-44** | `logger.rs`| LogFormat Default | Verifies LogFormat default is Text | Default derive | `LogFormat::Text` | `Text` | **PASSED** |
| **TC-UNIT-45** | `logger.rs`| LogFormat Serde | Tests serialization & deserialization | TOML `format = "json"` | Deserializes to `Json` | Matches | **PASSED** |
| **TC-UNIT-46** | `logger.rs`| LogFormat Variants | Checks inequality of variants | `Text != Json` | `true` | `true` | **PASSED** |
| **TC-UNIT-47** | `server.rs`| TokenBucket Initial State | Verifies token capacity on creation | Rate 5 | Allows 5 immediate tokens | 5 allowed | **PASSED** |
| **TC-UNIT-48** | `server.rs`| TokenBucket Exhaustion | Blocks after tokens exhausted | 6th attempt on rate 5 | Returns `false` | `false` | **PASSED** |
| **TC-UNIT-49** | `server.rs`| TokenBucket Refill | Refills tokens after elapsed time | Advance `last_refill` by 1s | Returns `true` | `true` | **PASSED** |
| **TC-UNIT-50** | `server.rs`| TokenBucket Max Capping | Does not exceed max micro-tokens | Advance `last_refill` by 10s | Capped at rate * 1M | Capped | **PASSED** |
| **TC-UNIT-51** | `server.rs`| TokenBucket Partial Refill | Partial time gives proportional tokens | Advance by 500ms | Allowed 4-6 tokens | Allowed 5 | **PASSED** |
| **TC-UNIT-52** | `server.rs`| Per-IP Guard Decrement | Drop guard decrements active count | Single connection guard | Count decrements to 0 | Key removed | **PASSED** |
| **TC-UNIT-53** | `server.rs`| Per-IP Guard Multiple Conns | Tracks multiple connections per IP | 2 connection guards | Drops decrement correctly | Matches | **PASSED** |
| **TC-UNIT-54** | `server.rs`| Per-IP Guard Independence | Different IPs tracked separately | IP A and IP B guards | Dropping A preserves B | Preserved | **PASSED** |
| **TC-UNIT-55** | `server.rs`| Per-IP Mutex Poison Recovery| Survives panic inside locked section | Poison mutex via panic | Guard accesses via `into_inner()` | Recovered | **PASSED** |
| **TC-UNIT-56** | `server.rs`| Metrics Initial Counters | Verifies counters start at zero | `Metrics::new()` | All atomic counters 0 | All 0 | **PASSED** |
| **TC-UNIT-57** | `server.rs`| Metrics Atomic Increments | Tests atomic addition & Prometheus | `fetch_add` across metrics | `format_prometheus()` output | Matches | **PASSED** |
| **TC-UNIT-58** | `server.rs`| Cert Expiry Metric Format | Emits expiry timestamp gauge | `cert_expiry = 1763553600` | Contains gauge line | Gauge present | **PASSED** |
| **TC-UNIT-59** | `server.rs`| Immediate Disconnect (Hold=False)| Closes connection post-greeting | `hold_open = false` | Exits in < 500ms post banner | Exited immediately | **PASSED** |
| **TC-UNIT-60** | `server.rs`| Hold-Open EOF Detection | Detects peer disconnect immediately | `hold_open = true`, peer closes | Detects EOF, exits < 2s | Exited < 2s | **PASSED** |
| **TC-UNIT-61** | `server.rs`| Rejection Banner Delivery | Transmits error message before close | `spawn_rejection` | Client receives error banner | Banner received | **PASSED** |
| **TC-UNIT-62** | `server.rs`| Silent Reject Drop | Drops without transmitting bytes | `silent_reject = true` | Client receives 0 bytes | 0 bytes | **PASSED** |
| **TC-UNIT-63** | `server.rs`| CIDR Forbidden Banner | Transmits forbidden message | `silent_reject = false` | Receives `FORBIDDEN_RESPONSE` | Banner received | **PASSED** |
| **TC-UNIT-64** | `server.rs`| Payload Drain Cap (Hold-Open)| Terminates connection on 16KB drain | Client streams 32KB payload | Closes socket within 1s | Closed | **PASSED** |
| **TC-UNIT-65** | `server.rs`| Post-Drain Slot Reclamation | Frees connection slot after drain cap | Send 18KB then drop | Frees slot immediately | Slot freed | **PASSED** |
| **TC-UNIT-66** | `server.rs`| Client Healthcheck Success | Healthcheck probe connects and reads | Remote mock listener | `run_health_check` is `Ok` | `Ok(())` | **PASSED** |
| **TC-UNIT-67** | `server.rs`| Client Healthcheck on Rate Limit | Fails healthcheck on rate limit banner | Server returns rate limit banner | Returns `Err(unexpected banner)` | `Err` | **PASSED** |
| **TC-UNIT-68** | `server.rs`| Client Healthcheck on CIDR Drop | Fails healthcheck on forbidden banner | Server returns CIDR banner | Returns `Err` | `Err` | **PASSED** |
| **TC-UNIT-69** | `server.rs`| Constant-Time Token Matching | Verifies matching tokens return true | Identical token strings | `true` | `true` | **PASSED** |
| **TC-UNIT-70** | `server.rs`| Constant-Time Token Mismatch | Verifies mismatched tokens return false | Different token strings | `false` | `false` | **PASSED** |
| **TC-UNIT-71** | `server.rs`| Constant-Time Length Invariant | Handles different-length strings | 6 chars vs 20 chars | `false` | `false` | **PASSED** |
| **TC-UNIT-72** | `server.rs`| IPv6 Subnet Key Masking | Aggregates IPv6 to /64 routing prefix | 2 IPs in same /64 | Same `rate_limit_key` | Same key | **PASSED** |
| **TC-UNIT-73** | `server.rs`| Per-IP Limiter Isolation | Throttles aggressive IP without peer starvation | IP A bursts, IP B connects | IP A blocked, IP B allowed | Allowed | **PASSED** |
| **TC-UNIT-74** | `server.rs`| Multi-Port Listener Creation | Binds multiple ports simultaneously | Ports 1 and 2 | Accepts connections on both | Accepted | **PASSED** |
| **TC-UNIT-75** | `server.rs`| Admin Server GET /healthz | Scrapes health endpoint | `GET /healthz` | HTTP 200 OK "OK\n" | 200 OK | **PASSED** |
| **TC-UNIT-76** | `server.rs`| Admin Server HEAD /healthz | Scrapes head health endpoint | `HEAD /healthz` | HTTP 200 OK (empty body) | 200 OK | **PASSED** |
| **TC-UNIT-77** | `server.rs`| Admin Server GET /metrics | Scrapes Prometheus endpoint | `GET /metrics` | HTTP 200 with Prometheus text | 200 OK | **PASSED** |
| **TC-UNIT-78** | `server.rs`| Admin Server Bearer Rejection | Rejects unauthenticated request | No Authorization header | HTTP 401 Unauthorized | 401 | **PASSED** |
| **TC-UNIT-79** | `server.rs`| Admin Server Bearer Success | Accepts request with valid token | `Authorization: Bearer <token>` | HTTP 200 OK | 200 OK | **PASSED** |
| **TC-UNIT-80** | `server.rs`| Admin Header Fragmentation | Reassembles streaming TCP packets | Split `GET /metrics` in 2 chunks | Reassembles, returns 200 OK | 200 OK | **PASSED** |
| **TC-UNIT-81** | `server.rs`| Admin Header Overflow Defense| Drops header exceeding 4096 bytes | Stream 4100 header bytes | HTTP 431 Fields Too Large | 431 | **PASSED** |
| **TC-UNIT-82** | `server.rs`| Admin Idle Read Timeout | Drops connection sending 0 bytes | Connect and remain idle | Terminates connection in ~1s | Closed | **PASSED** |
| **TC-UNIT-83** | `server.rs`| Dual-Stack IPv6 Socket | Binds IPv6 socket accepting IPv4 | `[::]:0` with dual-stack | Accepts IPv4 mapped connection | Accepted | **PASSED** |
| **TC-UNIT-84** | `server.rs`| Unified IPv4 Socket Creation | Verifies socket creation & non-blocking | `127.0.0.1:0` | Port allocated, non-blocking | Created | **PASSED** |

---

### 2.2 External Integration & Networking Tests (`tests/` - 55 Cases)

| Test ID | Test Suite | Test Objective | Preconditions | Input / Steps | Expected Result | Actual Result | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **TC-EXT-UNIT-01** | `unit_tests.rs` | Default CLI Resolution | CLI instance with `None` fields | `cli.resolve_defaults()` | Bind `127.0.0.1`, max_conns `100` | Fields resolved | **PASSED** |
| **TC-EXT-UNIT-02** | `unit_tests.rs` | Multi-Port Range Expansion | Overlapping ranges and discrete | `"8080, 9000-9003, 8080"` | `[8080, 9000..9003]` | Sorted & deduped | **PASSED** |
| **TC-EXT-UNIT-03** | `unit_tests.rs` | Range Limit Check (>100) | Port range spanning 151 ports | `"1000-1150"` | `resolve_all_ports` returns `Err` | `Err` (Rejected) | **PASSED** |
| **TC-EXT-UNIT-04** | `unit_tests.rs` | Port Count Cap (>50) | Total discrete ports exceed 50 | `"1000-1060"` | `resolve_all_ports` returns `Err` | `Err` (Rejected) | **PASSED** |
| **TC-EXT-UNIT-05** | `unit_tests.rs` | Limit Conflict Validation | `per_ip > global` connection cap | `max=10, per_ip=20` | `validate()` returns `Err` | `Err` (Rejected) | **PASSED** |
| **TC-EXT-UNIT-06** | `unit_tests.rs` | Rate Conflict Validation | `per_ip_rate > global_rate` | `rate=10, per_ip=15` | `validate()` returns `Err` | `Err` (Rejected) | **PASSED** |
| **TC-EXT-UNIT-07** | `unit_tests.rs` | IPv4 CIDR Boundary Check | Check subnet mask boundaries | `192.168.1.0/24` | Contains .0, .1, .254, .255 | Correct matching | **PASSED** |
| **TC-EXT-UNIT-08** | `unit_tests.rs` | IPv6 CIDR Boundary Check | Check IPv6 /16 subnet mask | `fd00::/16` | Contains fd00::1, fd00:ffff::1 | Correct matching | **PASSED** |
| **TC-EXT-UNIT-09** | `unit_tests.rs` | Invalid Prefix Lengths | Invalid prefix bounds checking | `10.0.0.1/33`, `::1/129` | `Cidr::parse` returns `Err` | `Err` (Rejected) | **PASSED** |
| **TC-EXT-UNIT-10** | `unit_tests.rs` | Token Bucket Rapid Acquire | Consume tokens sequentially | Rate 3 | First 3 return true, 4th false | Matches | **PASSED** |
| **TC-EXT-UNIT-11** | `unit_tests.rs` | IPv6 Key Prefix Aggregation| Compare IPv6 routing keys | 2 addresses in same /64 | `rate_limit_key` equal | Equal | **PASSED** |
| **TC-EXT-UNIT-12** | `unit_tests.rs` | Per-IP Limiter Capacity | 100 different IPs acquire | Rate 10 per IP | All 100 IPs acquire cleanly | All acquired | **PASSED** |
| **TC-EXT-UNIT-13** | `unit_tests.rs` | Constant-Time Equality | Verify token comparison matrix | Valid vs wrong vs empty | Exact boolean outputs | Matches | **PASSED** |
| **TC-INT-01** | `integration_tests` | Daemon Startup & Single Probe| Start full in-process daemon | Port allocated, client connects | Receives `RESPONSE`, healthcheck Ok | `Ok(())` | **PASSED** |
| **TC-INT-02** | `integration_tests` | Closed Port Probe Rejection | Probe closed TCP port | Local unallocated port | `run_health_check` returns `Err` | `Err` (Connection refused) | **PASSED** |
| **TC-INT-03** | `integration_tests` | Admin Endpoints Full Matrix | Test GET/HEAD healthz, metrics | Bearer token auth enabled | 200 with token, 401 without | All match expected | **PASSED** |
| **TC-INT-04** | `integration_tests` | Daemon Stop & Restart Cycle | Stop daemon and restart on port | Port P, stop, restart on P | Both cycles accept probes | Clean restart | **PASSED** |
| **TC-NET-01** | `networking_tests` | Successful Plain TCP Conn | Connect to mock listener | Local loopback port | Reads `RESPONSE` banner | `RESPONSE` read | **PASSED** |
| **TC-NET-02** | `networking_tests` | Connection Refused Scenario | Connect to closed port | Port without listener | Immediate OS error (10061/ECONNREFUSED) | Immediate refusal | **PASSED** |
| **TC-NET-03** | `networking_tests` | Connection Read Timeout | Server accepts and stays idle | 100ms read timeout | Timeout elapsed error | Timed out | **PASSED** |
| **TC-NET-04** | `networking_tests` | Connection Reset by Peer | Server accepts and drops socket | Reset mock server | Detects EOF (0 bytes) | EOF detected | **PASSED** |
| **TC-NET-05** | `networking_tests` | Slow Endpoint Latency | Server delays response by 200ms | 50ms timeout vs 500ms timeout | 50ms fails, 500ms succeeds | Matches expected | **PASSED** |
| **TC-NET-06** | `networking_tests` | Multi-Port Concurrent Probing| Daemon listening on 3 ports | 3 distinct dynamic ports | All 3 ports return `RESPONSE` | 3/3 succeeded | **PASSED** |
| **TC-NET-07** | `networking_tests` | Mixed Endpoints Probe List | Array of alive and dead targets | `[alive_addr, dead_addr]` | Output `[true, false]` | `[true, false]` | **PASSED** |
| **TC-NET-08** | `networking_tests` | TLS Transport Reachability | TLS 1.3 server with CA cert | Plain vs TLS probe | Plain fails, TLS succeeds | Matches expected | **PASSED** |
| **TC-NET-09** | `networking_tests` | mTLS Client Certificate Auth | mTLS server enforcing client CA | Client without cert | Handshake rejected | Handshake failed | **PASSED** |
| **TC-CONC-01**| `concurrency_tests` | Staged Concurrency Scaling | Concurrency: 1, 10, 50, 100, 250| Simultaneous barrier dispatch | 100% success rate across all stages | 100% success (up to 4.7k ops/s) | **PASSED** |
| **TC-CONC-02**| `concurrency_tests` | Global Connection Cap | `max_connections = 8` | 8 hold open + 1 overflow | Overflow receives `CONNECTION_LIMIT_RESPONSE` | Received limit banner | **PASSED** |
| **TC-CONC-03**| `concurrency_tests` | Per-IP Connection Cap | `max_connections_per_ip = 4` | 4 hold open + 1 overflow | Overflow receives `PER_IP_LIMIT_RESPONSE` | Received per-IP banner | **PASSED** |
| **TC-CONC-04**| `concurrency_tests` | Global Rate Limiter Burst | `max_rate = 5` | Burst 15 requests immediately | Rejection of excess requests | Rate limited > 0 | **PASSED** |
| **TC-CFG-01** | `configuration_tests`| Valid Configuration Load | Load `valid.toml` fixture | Path to fixture file | All fields populated & validated | Loaded cleanly | **PASSED** |
| **TC-CFG-02** | `configuration_tests`| Malformed TOML Rejection | Load `malformed.toml` fixture | Unclosed string syntax | Returns TOML parse error | Parse error | **PASSED** |
| **TC-CFG-03** | `configuration_tests`| Nonexistent Config File | Load missing file path | Path to non-existent file | Returns "file not found" error | File not found | **PASSED** |
| **TC-CFG-04** | `configuration_tests`| Port Range > 100 Rejection | Load `invalid_ranges.toml` | Range `1000-1200` | Rejection: exceeds maximum range 100 | Rejected | **PASSED** |
| **TC-CFG-05** | `configuration_tests`| Malformed CIDR Rejection | Load `invalid_cidr.toml` | `10.0.0.1/35` | Returns CIDR validation error | Rejected | **PASSED** |
| **TC-CFG-06** | `configuration_tests`| Conflicting Limits Rejection | Load `conflicting_limits.toml` | `max=10, per_ip=50` | Rejection: per-IP exceeds global | Rejected | **PASSED** |
| **TC-CFG-07** | `configuration_tests`| Conflicting Rate Rejection | Load `conflicting_rate.toml` | `rate=10, per_ip_rate=25` | Rejection: per-IP rate exceeds global | Rejected | **PASSED** |
| **TC-CFG-08** | `configuration_tests`| TLS Missing Key Rejection | Load `tls_missing_key.toml` | Cert without key | Rejection: both cert & key required | Rejected | **PASSED** |
| **TC-CFG-09** | `configuration_tests`| Precedence: CLI Overrides File| Pass `--bind 10.10.10.10` | Explicit CLI with `valid.toml` | CLI value preserved; file fills rest | Preserved | **PASSED** |
| **TC-CLI-01** | `cli_tests` | CLI Help Output | Execute binary `--help` | Subprocess `--help` | Exits 0, prints usage & flags | Exited 0 | **PASSED** |
| **TC-CLI-02** | `cli_tests` | CLI Version Output | Execute binary `--version` | Subprocess `--version` | Exits 0, prints `portprobe 2.0.0` | Exited 0 | **PASSED** |
| **TC-CLI-03** | `cli_tests` | Missing Port Error Code | Execute binary with no args | Subprocess empty args | Exits non-zero, prints port error | Exited non-zero | **PASSED** |
| **TC-CLI-04** | `cli_tests` | Invalid Port Number | Execute binary `--port 99999` | Out-of-range port | Exits non-zero, clap error | Exited non-zero | **PASSED** |
| **TC-CLI-05** | `cli_tests` | Invalid Timeout Number | Execute binary `--timeout 0` | Timeout 0 | Exits non-zero, validation error | Exited non-zero | **PASSED** |
| **TC-CLI-06** | `cli_tests` | Unknown Flag Rejection | Execute binary `--unknown-flag` | Unrecognized CLI argument | Exits non-zero, unknown arg error | Exited non-zero | **PASSED** |
| **TC-CLI-07** | `cli_tests` | Client Healthcheck Success | Execute `--health-check` on mock | Mock server running | Exits 0, prints SUCCESS banner | Exited 0 | **PASSED** |
| **TC-CLI-08** | `cli_tests` | Client Healthcheck Failure | Execute `--health-check` closed | Closed local port | Exits 1, prints FAILURE banner | Exited 1 | **PASSED** |
| **TC-CLI-09** | `cli_tests` | Windows Service Flag Handled | Execute binary `--service` | Flag passed on CLI | Exits non-zero with guidance message | Exited non-zero | **PASSED** |
| **TC-REL-01** | `reliability_tests` | 300 Sequential Probes | Run 300 sequential probes | Running daemon | 300/300 success, zero socket leaks | 300/300 succeeded | **PASSED** |
| **TC-REL-02** | `reliability_tests` | 50 Rapid Disconnects | 50 clients connect and drop | Hold-open daemon | Sockets closed, daemon fully responsive | Slots freed | **PASSED** |
| **TC-REL-03** | `reliability_tests` | Graceful Shutdown Under Load | Fire shutdown with 10 clients open | 10 active connections | Completes in < 3s without panic | Clean shutdown | **PASSED** |
| **TC-REL-04** | `reliability_tests` | Payload Drain Cap (20KB) | Client streams 20KB garbage | Hold-open mode enabled | Server drops connection on 16KB cap | Dropped < 2s | **PASSED** |
| **TC-SEC-01** | `security_tests` | Silent Reject vs Non-Silent | Unauthorized client from loopback | `allow_cidr = ["10.0.0.0/8"]` | Silent mode: 0 bytes; non-silent: banner | Exact match | **PASSED** |
| **TC-SEC-02** | `security_tests` | Slowloris 4100-byte Header Flooding| Send 4100 bytes without `\r\n\r\n` | Admin endpoint | HTTP 431 Request Header Fields Too Large | 431 returned | **PASSED** |
| **TC-SEC-03** | `security_tests` | Constant-Time Token Properties | Test identical vs early mismatch | Timing invariant verification | Both evaluated without branch short-circuit | Evaluated | **PASSED** |
