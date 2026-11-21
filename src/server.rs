//! TCP server implementation for PortProbe.
//!
//! # Security features
//!
//! - **Token bucket rate limiting**: Smooth connection rate enforcement
//!   without the burst vulnerability of fixed-window approaches.
//! - **Global connection cap**: Semaphore-based limit on total concurrent
//!   connection handler tasks.
//! - **Per-IP connection cap**: Prevents a single source from monopolizing
//!   all connection slots, mitigating single-source DoS attacks.
//! - **Configurable connection lifetime**: Connections are automatically
//!   closed after the configured timeout.
//! - **Graceful shutdown**: Ctrl+C / SIGTERM notifies all handlers and waits for
//!   them to finish before exiting.
//! - **Immediate Slot Reclamation (Anti-Slowloris)**: Default one-shot probe mode
//!   closes connection immediately post-response unless `--hold-open` is enabled.
//! - **Fixed response**: Only a hardcoded message is sent; no client
//!   input is processed or executed.
//! - **Connection metrics**: Atomic counters track connection lifecycle
//!   events, logged as a summary on shutdown and scrapable via Prometheus `/metrics`.
//! - **TLS / mTLS Encryption**: Pure-Rust TLS 1.3 encryption with client certificate verification.
//! - **POSIX Privilege Dropping**: Drops root privileges post-socket-bind on Linux/Unix with safe EUID pre-check.
//! - **Windows Socket Hijacking Hardening**: Enforces `set_reuse_address(false)` across IPv4 and IPv6.
//! - **Administrative Metrics & Health Endpoint**: Localhost `/healthz` and `/metrics` with read timeouts and concurrency limits.
//! - **Dynamic SIGHUP Configuration Reload**: Live reload of CIDR subnet rules on Unix.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::{watch, RwLock, Semaphore};
use tokio::task::JoinSet;
use tokio::time::{sleep, sleep_until, timeout_at, Instant};
use tokio_rustls::rustls::server::ServerConfig;
use tokio_rustls::rustls::RootCertStore;
use tokio_rustls::TlsAcceptor;

use tracing::{debug, error, info, warn};

use crate::cli::Cidr;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The fixed response sent to every connected client.
pub const RESPONSE: &[u8] = b"PORTPROBE: TCP CONNECTION SUCCESSFUL\n";

/// Error response sent to clients rejected due to rate limiting.
pub const RATE_LIMITED_RESPONSE: &[u8] = b"PORTPROBE: RATE LIMIT EXCEEDED\n";

/// Error response sent to clients rejected due to the global concurrent connection cap.
pub const CONNECTION_LIMIT_RESPONSE: &[u8] = b"PORTPROBE: CONNECTION LIMIT REACHED\n";

/// Error response sent to clients rejected because their source IP already has too many active connections.
pub const PER_IP_LIMIT_RESPONSE: &[u8] = b"PORTPROBE: PER-IP CONNECTION LIMIT REACHED\n";

/// Error response sent to clients rejected by the CIDR subnet filter.
pub const FORBIDDEN_RESPONSE: &[u8] = b"PORTPROBE: CONNECTION NOT ALLOWED (CIDR FILTER)\n";

/// Maximum duration to wait when attempting to send an error response to a rejected client.
const REJECTION_WRITE_TIMEOUT: Duration = Duration::from_millis(500);

/// Maximum concurrent background rejection tasks.
const MAX_CONCURRENT_REJECTIONS: usize = 32;

/// Maximum duration to wait when transmitting the initial success greeting banner or completing TLS handshake.
const GREETING_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum unexpected payload bytes drained from a client during the hold-open phase.
const MAX_HOLD_OPEN_DRAIN_BYTES: usize = 16 * 1024;

/// Delay before retrying after an `accept()` error.
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(200);

// ---------------------------------------------------------------------------
// Unified Connection Stream (Plain TCP, TLS Server, or TLS Client)
// ---------------------------------------------------------------------------

/// Encapsulates plain TCP, incoming TLS server stream, or outgoing TLS client probe.
pub enum ConnectionStream {
    Plain(TcpStream),
    TlsServer(Box<tokio_rustls::server::TlsStream<TcpStream>>),
    TlsClient(Box<tokio_rustls::client::TlsStream<TcpStream>>),
}

impl AsyncRead for ConnectionStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ConnectionStream::Plain(s) => Pin::new(s).poll_read(cx, buf),
            ConnectionStream::TlsServer(s) => Pin::new(s.as_mut()).poll_read(cx, buf),
            ConnectionStream::TlsClient(s) => Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ConnectionStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ConnectionStream::Plain(s) => Pin::new(s).poll_write(cx, buf),
            ConnectionStream::TlsServer(s) => Pin::new(s.as_mut()).poll_write(cx, buf),
            ConnectionStream::TlsClient(s) => Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ConnectionStream::Plain(s) => Pin::new(s).poll_flush(cx),
            ConnectionStream::TlsServer(s) => Pin::new(s.as_mut()).poll_flush(cx),
            ConnectionStream::TlsClient(s) => Pin::new(s.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ConnectionStream::Plain(s) => Pin::new(s).poll_shutdown(cx),
            ConnectionStream::TlsServer(s) => Pin::new(s.as_mut()).poll_shutdown(cx),
            ConnectionStream::TlsClient(s) => Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}

// ---------------------------------------------------------------------------
// Security & Authentication Helpers
// ---------------------------------------------------------------------------

/// Constant-time Bearer token verification using SHA-256 pre-hashing and constant-time digest comparison.
pub fn constant_time_token_verify(candidate: &str, expected: &str) -> bool {
    use ring::digest::{digest, SHA256};
    use subtle::ConstantTimeEq;

    let candidate_digest = digest(&SHA256, candidate.as_bytes());
    let expected_digest = digest(&SHA256, expected.as_bytes());
    candidate_digest
        .as_ref()
        .ct_eq(expected_digest.as_ref())
        .into()
}

/// Extract expiration timestamp in UTC epoch seconds from raw DER X.509 certificate bytes.
pub fn parse_cert_expiry_epoch_seconds(der: &[u8]) -> Option<u64> {
    let mut i = 0;
    let mut times_found = 0;
    while i + 2 < der.len() {
        let tag = der[i];
        let len = der[i + 1] as usize;
        if (tag == 0x17 && len == 13) || (tag == 0x18 && len == 15) {
            times_found += 1;
            if times_found == 2 {
                // Second time element in TBSCertificate is notAfter
                let time_str = std::str::from_utf8(&der[i + 2..i + 2 + len]).ok()?;
                return parse_asn1_time_to_epoch(time_str);
            }
            i += 2 + len;
        } else {
            i += 1;
        }
    }
    None
}

fn parse_asn1_time_to_epoch(time_str: &str) -> Option<u64> {
    let s = time_str.strip_suffix('Z').unwrap_or(time_str);
    let (year, rest) = if s.len() == 12 {
        let yy: i32 = s[0..2].parse().ok()?;
        let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
        (year, &s[2..])
    } else if s.len() == 14 {
        let yyyy: i32 = s[0..4].parse().ok()?;
        (yyyy, &s[4..])
    } else {
        return None;
    };
    if rest.len() < 10 {
        return None;
    }
    let month: u8 = rest[0..2].parse().ok()?;
    let day: u8 = rest[2..4].parse().ok()?;
    let hour: u8 = rest[4..6].parse().ok()?;
    let min: u8 = rest[6..8].parse().ok()?;
    let sec: u8 = rest[8..10].parse().ok()?;

    let month_enum = match month {
        1 => time::Month::January,
        2 => time::Month::February,
        3 => time::Month::March,
        4 => time::Month::April,
        5 => time::Month::May,
        6 => time::Month::June,
        7 => time::Month::July,
        8 => time::Month::August,
        9 => time::Month::September,
        10 => time::Month::October,
        11 => time::Month::November,
        12 => time::Month::December,
        _ => return None,
    };

    let date = time::Date::from_calendar_date(year, month_enum, day).ok()?;
    let time = time::Time::from_hms(hour, min, sec).ok()?;
    let dt = time::PrimitiveDateTime::new(date, time).assume_utc();
    let epoch = dt.unix_timestamp();
    if epoch >= 0 {
        Some(epoch as u64)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// TLS Configuration Loader
// ---------------------------------------------------------------------------

/// Load X.509 certificates and private key from PEM files and initialize a Rustls `TlsAcceptor`.
pub fn load_tls_acceptor(
    cert_path: &Path,
    key_path: &Path,
    client_ca_path: Option<&Path>,
    tls_min_version: Option<&str>,
) -> Result<(TlsAcceptor, Option<u64>), Box<dyn std::error::Error + Send + Sync>> {
    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

    crate::cli::verify_file_permissions(key_path)
        .map_err(|e| format!("TLS private key file security check failed: {}", e))?;

    let certs: Vec<CertificateDer<'static>> =
        CertificateDer::pem_file_iter(cert_path)?.collect::<Result<Vec<_>, _>>()?;

    if certs.is_empty() {
        return Err("No certificates found in TLS certificate file".into());
    }

    let expiry_timestamp = parse_cert_expiry_epoch_seconds(certs[0].as_ref());
    if let Some(exp) = expiry_timestamp {
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if exp.saturating_sub(now_epoch) < 30 * 86400 {
            warn!(
                "TLS certificate will expire in less than 30 days! (Expires at epoch {})",
                exp
            );
        }
    }

    let key: PrivateKeyDer<'static> = PrivateKeyDer::from_pem_file(key_path)?;

    let protocol_versions: &[&tokio_rustls::rustls::SupportedProtocolVersion] =
        if tls_min_version == Some("1.3") {
            info!("TLS 1.3-only protocol enforcement enabled");
            &[&tokio_rustls::rustls::version::TLS13]
        } else {
            &[
                &tokio_rustls::rustls::version::TLS13,
                &tokio_rustls::rustls::version::TLS12,
            ]
        };

    let server_config = if let Some(ca_path) = client_ca_path {
        let mut root_store = RootCertStore::empty();
        for ca in CertificateDer::pem_file_iter(ca_path)? {
            root_store.add(ca?)?;
        }
        let client_verifier =
            tokio_rustls::rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
                .build()?;
        ServerConfig::builder_with_protocol_versions(protocol_versions)
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(certs, key)?
    } else {
        ServerConfig::builder_with_protocol_versions(protocol_versions)
            .with_no_client_auth()
            .with_single_cert(certs, key)?
    };

    Ok((TlsAcceptor::from(Arc::new(server_config)), expiry_timestamp))
}

// ---------------------------------------------------------------------------
// Privilege Dropping (POSIX / Unix)
// ---------------------------------------------------------------------------

/// Drop elevated privileges on Unix systems post-socket-bind.
/// Safely skips drop if the process already executes as an unprivileged user,
/// but strictly aborts if running as root (EUID 0) and privilege drop fails.
#[cfg(unix)]
pub fn drop_privileges(user: Option<&str>, group: Option<&str>) -> Result<(), String> {
    if user.is_none() && group.is_none() {
        return Ok(());
    }
    // Safe POSIX user ID check without raw FFI blocks
    let is_root = nix::unistd::geteuid().is_root();
    let mut pd = privdrop::PrivDrop::default();
    if let Some(u) = user {
        pd = pd.user(u);
    }
    if let Some(g) = group {
        pd = pd.group(g);
    }
    match pd.apply() {
        Ok(()) => {
            info!(
                "Successfully dropped privileges to user: {:?}, group: {:?}",
                user, group
            );
            Ok(())
        }
        Err(e) => {
            if is_root {
                Err(format!(
                    "CRITICAL: Running as root (EUID 0) but failed to drop privileges to user {:?}, group {:?}: {}",
                    user, group, e
                ))
            } else {
                warn!(
                    "Privilege drop skipped: process is already running as an unprivileged user ({})",
                    e
                );
                Ok(())
            }
        }
    }
}

#[cfg(not(unix))]
pub fn drop_privileges(user: Option<&str>, group: Option<&str>) -> Result<(), String> {
    if user.is_some() || group.is_some() {
        warn!("Privilege dropping via --user/--group is not supported on Windows. Use Windows Service accounts or ACLs.");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Client Healthcheck
// ---------------------------------------------------------------------------

/// Configuration for TLS-enabled healthcheck probes.
#[derive(Debug, Clone)]
pub struct HealthCheckTlsConfig {
    pub domain: String,
    pub ca_cert: Option<PathBuf>,
}

/// Connect to a running PortProbe instance as a health probe client.
/// Strictly validates against `RESPONSE` to eliminate false positives.
pub async fn run_health_check(
    addr: SocketAddr,
    tls_config: Option<HealthCheckTlsConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stream = tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(addr)).await??;
    let mut conn: ConnectionStream = match tls_config {
        Some(tls_cfg) => {
            let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();
            let mut root_store = RootCertStore::empty();
            if let Some(ref ca_path) = tls_cfg.ca_cert {
                for ca in CertificateDer::pem_file_iter(ca_path)? {
                    root_store.add(ca?)?;
                }
            }
            let client_config = tokio_rustls::rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
            let server_name = rustls_pki_types::ServerName::try_from(tls_cfg.domain)?.to_owned();
            let tls_stream = tokio::time::timeout(
                Duration::from_secs(3),
                connector.connect(server_name, stream),
            )
            .await??;
            ConnectionStream::TlsClient(Box::new(tls_stream))
        }
        None => ConnectionStream::Plain(stream),
    };

    let mut buf = [0u8; 64];
    let n = tokio::time::timeout(Duration::from_secs(3), conn.read(&mut buf)).await??;
    if n > 0 && buf[..n].starts_with(RESPONSE) {
        info!(
            "Healthcheck passed: received valid success banner from {}",
            addr
        );
        Ok(())
    } else {
        let msg = String::from_utf8_lossy(&buf[..n]);
        Err(format!(
            "Healthcheck rejected: received non-success banner: {:?}",
            msg.trim()
        )
        .into())
    }
}

// ---------------------------------------------------------------------------
// Token bucket rate limiter
// ---------------------------------------------------------------------------

const MICRO_TOKENS_PER_TOKEN: u128 = 1_000_000;

pub struct TokenBucket {
    micro_tokens: u128,
    max_micro_tokens: u128,
    refill_rate: u64,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(rate: u64) -> Self {
        let max_micro_tokens = (rate as u128) * MICRO_TOKENS_PER_TOKEN;
        Self {
            micro_tokens: max_micro_tokens,
            max_micro_tokens,
            refill_rate: rate,
            last_refill: Instant::now(),
        }
    }

    pub fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let elapsed_nanos = now.duration_since(self.last_refill).as_nanos();

        let added_micro_tokens = (elapsed_nanos * (self.refill_rate as u128)) / 1000;

        if added_micro_tokens > 0 {
            let new_micro_tokens = self.micro_tokens.saturating_add(added_micro_tokens);
            if new_micro_tokens >= self.max_micro_tokens {
                self.micro_tokens = self.max_micro_tokens;
                self.last_refill = now;
            } else {
                self.micro_tokens = new_micro_tokens;
                let nanos_consumed = (added_micro_tokens * 1000) / (self.refill_rate as u128);
                self.last_refill += Duration::from_nanos(nanos_consumed as u64);
            }
        }

        if self.micro_tokens >= MICRO_TOKENS_PER_TOKEN {
            self.micro_tokens -= MICRO_TOKENS_PER_TOKEN;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Per-IP rate limiter
// ---------------------------------------------------------------------------

/// Computes the rate-limiting tracking key. For IPv6, aggregates addresses to the /64 routing prefix
/// to prevent host rotation bypassing per-IP rate limits.
pub fn rate_limit_key(ip: IpAddr) -> IpAddr {
    match canonicalize_ip(ip) {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            IpAddr::V6(std::net::Ipv6Addr::new(
                segs[0], segs[1], segs[2], segs[3], 0, 0, 0, 0,
            ))
        }
    }
}

/// Tracks per-source-IP token buckets to prevent an aggressive client from
/// starving the global rate limit bucket.
pub struct PerIpRateLimiter {
    buckets: HashMap<IpAddr, TokenBucket>,
    rate_per_ip: u64,
    last_cleanup: Instant,
}

impl PerIpRateLimiter {
    pub fn new(rate_per_ip: u64) -> Self {
        Self {
            buckets: HashMap::new(),
            rate_per_ip,
            last_cleanup: Instant::now(),
        }
    }

    pub fn try_acquire(&mut self, ip: &IpAddr) -> bool {
        let key = rate_limit_key(*ip);
        let now = Instant::now();

        // Throttled periodic cleanup: only run at most once every 10 seconds to avoid linear scan CPU spikes
        if now.duration_since(self.last_cleanup) >= Duration::from_secs(10) {
            self.buckets.retain(|_, bucket| {
                now.duration_since(bucket.last_refill) < Duration::from_secs(60)
            });

            // Evict fully refilled buckets if still exceeding 5000 entries (protecting active cooldown entries)
            if self.buckets.len() > 5000 {
                let to_remove: Vec<IpAddr> = self
                    .buckets
                    .iter()
                    .filter(|(_, b)| b.micro_tokens >= b.max_micro_tokens)
                    .take(self.buckets.len() - 4000)
                    .map(|(k, _)| *k)
                    .collect();
                for k in to_remove {
                    self.buckets.remove(&k);
                }
            }
            self.last_cleanup = now;
        }

        let rate = self.rate_per_ip;
        let bucket = self
            .buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(rate));
        bucket.try_acquire()
    }
}

// ---------------------------------------------------------------------------
// Per-IP connection tracker
// ---------------------------------------------------------------------------

type PerIpTracker = Arc<Mutex<HashMap<IpAddr, usize>>>;

fn canonicalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                IpAddr::V4(v4)
            } else {
                IpAddr::V6(v6)
            }
        }
        IpAddr::V4(v4) => IpAddr::V4(v4),
    }
}

struct PerIpGuard {
    ip: IpAddr,
    tracker: PerIpTracker,
}

impl Drop for PerIpGuard {
    fn drop(&mut self) {
        let mut counts = self
            .tracker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(count) = counts.get_mut(&self.ip) {
            *count -= 1;
            if *count == 0 {
                counts.remove(&self.ip);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Connection metrics
// ---------------------------------------------------------------------------

pub struct Metrics {
    pub accepted: AtomicU64,
    pub rate_limited: AtomicU64,
    pub per_ip_rate_limited: AtomicU64,
    pub connection_limited: AtomicU64,
    pub per_ip_limited: AtomicU64,
    pub cidr_rejected: AtomicU64,
    pub completed: AtomicU64,
    pub timed_out: AtomicU64,
    pub errored: AtomicU64,
    pub tls_cert_expiry: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            accepted: AtomicU64::new(0),
            rate_limited: AtomicU64::new(0),
            per_ip_rate_limited: AtomicU64::new(0),
            connection_limited: AtomicU64::new(0),
            per_ip_limited: AtomicU64::new(0),
            cidr_rejected: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            timed_out: AtomicU64::new(0),
            errored: AtomicU64::new(0),
            tls_cert_expiry: AtomicU64::new(0),
        }
    }

    pub fn format_prometheus(&self) -> String {
        let expiry = self.tls_cert_expiry.load(Ordering::Relaxed);
        let expiry_line = if expiry > 0 {
            format!(
                "# HELP portprobe_tls_cert_expiry_timestamp_seconds TLS certificate expiration timestamp in UTC epoch seconds\n\
                 # TYPE portprobe_tls_cert_expiry_timestamp_seconds gauge\n\
                 portprobe_tls_cert_expiry_timestamp_seconds {}\n",
                expiry
            )
        } else {
            String::new()
        };

        format!(
            "{}\
             # HELP portprobe_accepted_total Total accepted connections\n\
             # TYPE portprobe_accepted_total counter\n\
             portprobe_accepted_total {}\n\
             # HELP portprobe_rate_limited_total Connections rejected by rate limiter\n\
             # TYPE portprobe_rate_limited_total counter\n\
             portprobe_rate_limited_total {}\n\
             # HELP portprobe_per_ip_rate_limited_total Connections rejected by per-IP rate limiter\n\
             # TYPE portprobe_per_ip_rate_limited_total counter\n\
             portprobe_per_ip_rate_limited_total {}\n\
             # HELP portprobe_connection_limited_total Connections rejected by global limit\n\
             # TYPE portprobe_connection_limited_total counter\n\
             portprobe_connection_limited_total {}\n\
             # HELP portprobe_per_ip_limited_total Connections rejected by per-IP limit\n\
             # TYPE portprobe_per_ip_limited_total counter\n\
             portprobe_per_ip_limited_total {}\n\
             # HELP portprobe_cidr_rejected_total Connections rejected by CIDR filter\n\
             # TYPE portprobe_cidr_rejected_total counter\n\
             portprobe_cidr_rejected_total {}\n\
             # HELP portprobe_completed_total Connections completed normally\n\
             # TYPE portprobe_completed_total counter\n\
             portprobe_completed_total {}\n\
             # HELP portprobe_timed_out_total Connections timed out during greeting\n\
             # TYPE portprobe_timed_out_total counter\n\
             portprobe_timed_out_total {}\n\
             # HELP portprobe_errored_total Connections with errors\n\
             # TYPE portprobe_errored_total counter\n\
             portprobe_errored_total {}\n",
            expiry_line,
            self.accepted.load(Ordering::Relaxed),
            self.rate_limited.load(Ordering::Relaxed),
            self.per_ip_rate_limited.load(Ordering::Relaxed),
            self.connection_limited.load(Ordering::Relaxed),
            self.per_ip_limited.load(Ordering::Relaxed),
            self.cidr_rejected.load(Ordering::Relaxed),
            self.completed.load(Ordering::Relaxed),
            self.timed_out.load(Ordering::Relaxed),
            self.errored.load(Ordering::Relaxed),
        )
    }

    pub fn log_summary(&self) {
        info!("====================================");
        info!("        CONNECTION METRICS");
        info!("====================================");
        info!(
            "Total accepted:       {}",
            self.accepted.load(Ordering::Relaxed)
        );
        info!(
            "Rate limited:         {}",
            self.rate_limited.load(Ordering::Relaxed)
        );
        info!(
            "Per-IP rate limited:  {}",
            self.per_ip_rate_limited.load(Ordering::Relaxed)
        );
        info!(
            "Connection limited:   {}",
            self.connection_limited.load(Ordering::Relaxed)
        );
        info!(
            "Per-IP limited:       {}",
            self.per_ip_limited.load(Ordering::Relaxed)
        );
        info!(
            "CIDR rejected:        {}",
            self.cidr_rejected.load(Ordering::Relaxed)
        );
        info!(
            "Completed normally:   {}",
            self.completed.load(Ordering::Relaxed)
        );
        info!(
            "Timed out (greeting): {}",
            self.timed_out.load(Ordering::Relaxed)
        );
        info!(
            "Errors:               {}",
            self.errored.load(Ordering::Relaxed)
        );
        info!("====================================");
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Server Execution & Admission Control
// ---------------------------------------------------------------------------

fn spawn_rejection(
    mut stream: TcpStream,
    message: &'static [u8],
    rejection_semaphore: Arc<Semaphore>,
) {
    let permit = match rejection_semaphore.try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return;
        }
    };

    tokio::spawn(async move {
        let _permit = permit;
        let _ = timeout_at(
            Instant::now() + REJECTION_WRITE_TIMEOUT,
            stream.write_all(message),
        )
        .await;
        let _ = stream.shutdown().await;
    });
}

#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => Some(s),
        Err(err) => {
            error!("Failed to register SIGTERM handler: {}", err);
            None
        }
    };

    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(s) => Some(s),
        Err(err) => {
            error!("Failed to register SIGINT handler: {}", err);
            None
        }
    };

    if sigterm.is_none() && sigint.is_none() {
        error!("No signal handlers could be registered; falling back to ctrl_c()");
        let _ = tokio::signal::ctrl_c().await;
        return;
    }

    tokio::select! {
        _ = async {
            if let Some(ref mut s) = sigterm {
                s.recv().await
            } else {
                std::future::pending().await
            }
        } => {
            warn!("SIGTERM received; initiating graceful shutdown...");
        }
        _ = async {
            if let Some(ref mut s) = sigint {
                s.recv().await
            } else {
                std::future::pending().await
            }
        } => {
            warn!("SIGINT received; initiating graceful shutdown...");
        }
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            warn!("Shutdown signal (Ctrl+C) received.");
        }
        Err(err) => {
            error!("Shutdown signal error: {}", err);
        }
    }
}

pub struct ServerParams {
    pub bind_ip: IpAddr,
    pub ports: Vec<u16>,
    pub timeout_seconds: u64,
    pub max_connections: usize,
    pub max_connections_per_ip: usize,
    pub max_rate: u64,
    pub max_rate_per_ip: u64,
    pub allow_cidrs: Vec<Cidr>,
    pub silent_reject: bool,
    pub hold_open: bool,
    pub tls_acceptor: Option<TlsAcceptor>,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub tls_client_ca_path: Option<PathBuf>,
    pub tls_min_version: Option<String>,
    pub cert_expiry: Option<u64>,
    pub admin_port: Option<u16>,
    pub admin_token: Option<String>,
    pub user: Option<String>,
    pub group: Option<String>,
    pub config_path: Option<PathBuf>,
}

pub struct ServerContext {
    pub semaphore: Arc<Semaphore>,
    pub per_ip_tracker: PerIpTracker,
    pub max_connections_per_ip: usize,
    pub connection_timeout: Duration,
    pub shutdown_tx: watch::Sender<bool>,
    pub metrics: Arc<Metrics>,
    pub allow_cidrs: Arc<RwLock<Vec<Cidr>>>,
    pub rejection_semaphore: Arc<Semaphore>,
    pub silent_reject: bool,
    pub hold_open: bool,
    pub tls_acceptor: Arc<RwLock<Option<TlsAcceptor>>>,
}

/// Hardened administrative HTTP server with read timeouts, concurrency limits, and Bearer token auth.
pub async fn start_admin_server(
    listener: TcpListener,
    addr: SocketAddr,
    metrics: Arc<Metrics>,
    admin_token: Option<String>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let admin_semaphore = Arc::new(Semaphore::new(16));
    let metrics_sub_semaphore = Arc::new(Semaphore::new(12)); // 4 permits always reserved for /healthz
    let admin_rate_limiter = Arc::new(Mutex::new(TokenBucket::new(20))); // 20 req/s rate limit

    info!(
        "Administrative health & metrics endpoint listening on http://{}",
        addr
    );
    if admin_token.is_none() {
        warn!(
            "Administrative endpoint listening without authentication. Configure --admin-token or PORTPROBE_ADMIN_TOKEN for production security."
        );
    } else {
        info!("Administrative endpoint Bearer token authentication enabled");
    }

    loop {
        tokio::select! {
            res = listener.accept() => {
                if let Ok((mut stream, remote_addr)) = res {
                    let metrics = Arc::clone(&metrics);
                    let admin_token = admin_token.clone();
                    let admin_rate_limiter = Arc::clone(&admin_rate_limiter);
                    let metrics_sub_semaphore = Arc::clone(&metrics_sub_semaphore);
                    let permit = match admin_semaphore.clone().try_acquire_owned() {
                        Ok(p) => p,
                        Err(_) => {
                            debug!("Admin endpoint connection limit reached; dropping client {}", remote_addr);
                            continue;
                        }
                    };

                    tokio::spawn(async move {
                        let _permit = permit;

                        // Rate limit admin requests
                        let is_rate_limited = {
                            let mut rl = admin_rate_limiter.lock().unwrap_or_else(|p| p.into_inner());
                            !rl.try_acquire()
                        };
                        if is_rate_limited {
                            let resp = "HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = stream.write_all(resp.as_bytes()).await;
                            return;
                        }

                        // Buffer headers until \r\n\r\n or \n\n with 4KB cap and 1.0s timeout
                        let mut header_buf = Vec::with_capacity(1024);
                        let mut temp_buf = [0u8; 512];
                        let header_deadline = Instant::now() + Duration::from_millis(1000);
                        let mut headers_complete = false;

                        while header_buf.len() < 4096 {
                            let remaining = header_deadline.saturating_duration_since(Instant::now());
                            if remaining.is_zero() {
                                break;
                            }
                            match tokio::time::timeout(remaining, stream.read(&mut temp_buf)).await {
                                Ok(Ok(0)) => break,
                                Ok(Ok(n)) => {
                                    header_buf.extend_from_slice(&temp_buf[..n]);
                                    if header_buf.windows(4).any(|w| w == b"\r\n\r\n")
                                        || header_buf.windows(2).any(|w| w == b"\n\n")
                                    {
                                        headers_complete = true;
                                        break;
                                    }
                                }
                                _ => break,
                            }
                        }

                        if !headers_complete && header_buf.len() >= 4096 {
                            let resp = "HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = stream.write_all(resp.as_bytes()).await;
                            let _ = stream.flush().await;
                            let _ = stream.shutdown().await;
                            let mut drain_buf = [0u8; 1024];
                            let drain_deadline = Instant::now() + Duration::from_millis(50);
                            while let Ok(Ok(n)) = tokio::time::timeout_at(drain_deadline, stream.read(&mut drain_buf)).await {
                                if n == 0 {
                                    break;
                                }
                            }
                            return;
                        }

                        if header_buf.is_empty() {
                            return;
                        }

                        let req_str = String::from_utf8_lossy(&header_buf);

                        if let Some(ref required_token) = admin_token {
                            let is_authorized = req_str.lines().any(|line| {
                                let lower = line.to_ascii_lowercase();
                                if lower.starts_with("authorization:") {
                                    let parts: Vec<&str> = line[14..].split_whitespace().collect();
                                    if parts.len() == 2 && parts[0].eq_ignore_ascii_case("bearer") {
                                        return constant_time_token_verify(parts[1], required_token);
                                    }
                                }
                                false
                            });

                            if !is_authorized {
                                warn!("Unauthorized administrative access attempt from {}", remote_addr);
                                let body = "401 Unauthorized: Invalid or missing Bearer token\n";
                                let resp = format!(
                                    "HTTP/1.1 401 Unauthorized\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nWWW-Authenticate: Bearer realm=\"portprobe\"\r\nConnection: close\r\n\r\n{}",
                                    body.len(), body
                                );
                                let _ = tokio::time::timeout(
                                    Duration::from_secs(1),
                                    stream.write_all(resp.as_bytes()),
                                )
                                .await;
                                return;
                            }
                        }

                        let is_get_healthz = req_str.starts_with("GET /healthz");
                        let is_head_healthz = req_str.starts_with("HEAD /healthz");

                        if is_get_healthz || is_head_healthz {
                            // Support both GET and HEAD methods for enterprise load balancers
                            let body = if is_head_healthz { "" } else { "OK\n" };
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 3\r\nConnection: close\r\n\r\n{}",
                                body
                            );
                            let _ = tokio::time::timeout(
                                Duration::from_secs(1),
                                stream.write_all(resp.as_bytes()),
                            )
                            .await;
                        } else if req_str.starts_with("GET /metrics") {
                            let _metrics_permit = match metrics_sub_semaphore.try_acquire() {
                                Ok(p) => p,
                                Err(_) => {
                                    let resp = "HTTP/1.1 503 Service Unavailable: Metrics Concurrency Cap Reached\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                                    let _ = stream.write_all(resp.as_bytes()).await;
                                    return;
                                }
                            };
                            let body = metrics.format_prometheus();
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(), body
                            );
                            let _ = tokio::time::timeout(
                                Duration::from_secs(1),
                                stream.write_all(resp.as_bytes()),
                            )
                            .await;
                        } else {
                            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = tokio::time::timeout(
                                Duration::from_secs(1),
                                stream.write_all(resp.as_bytes()),
                            )
                            .await;
                        }
                    });
                }
            }
            res = shutdown_rx.changed() => {
                if res.is_ok() && *shutdown_rx.borrow() {
                    break;
                }
            }
        }
    }
}

/// Start the TCP server and listen for connections until shutdown.
pub async fn start_server(
    params: ServerParams,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_server_with_shutdown(params, None).await
}

/// Start the TCP server with an optional external shutdown receiver for test automation and controlled lifecycles.
pub async fn start_server_with_shutdown(
    params: ServerParams,
    external_shutdown_rx: Option<watch::Receiver<bool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Synchronously bind administrative listener if configured
    let admin_binding = if let Some(admin_port) = params.admin_port {
        let addr = SocketAddr::from(([127, 0, 0, 1], admin_port));
        match TcpListener::bind(addr).await {
            Ok(l) => Some((l, addr)),
            Err(err) => {
                error!(
                    "Failed to bind administrative endpoint on {}: {}",
                    addr, err
                );
                return Err(format!(
                    "Failed to bind administrative endpoint on {}: {}",
                    addr, err
                )
                .into());
            }
        }
    } else {
        None
    };

    // Bind listeners across all configured ports
    let mut listeners = Vec::new();
    for &port in &params.ports {
        let bind_address = SocketAddr::new(params.bind_ip, port);
        let domain = if bind_address.is_ipv6() {
            socket2::Domain::IPV6
        } else {
            socket2::Domain::IPV4
        };
        let socket =
            socket2::Socket::new(domain, socket2::Type::STREAM, Some(socket2::Protocol::TCP))?;

        if bind_address.is_ipv6() {
            let _ = socket.set_only_v6(false);
        }

        #[cfg(not(windows))]
        socket.set_reuse_address(true)?;
        #[cfg(windows)]
        {
            // Enforce socket protection against port hijacking on Windows
            let _ = socket.set_reuse_address(false);
        }

        socket.bind(&bind_address.into())?;
        let backlog = (params.max_connections as i32).clamp(1024, 8192);
        socket.listen(backlog)?;
        socket.set_nonblocking(true)?;
        let listener = TcpListener::from_std(socket.into())?;
        listeners.push((listener, bind_address));
    }

    if let Err(err) = drop_privileges(params.user.as_deref(), params.group.as_deref()) {
        error!("Privilege drop failed: {}", err);
        return Err(err.into());
    }

    let semaphore = Arc::new(Semaphore::new(params.max_connections));
    let per_ip_tracker: PerIpTracker = Arc::new(Mutex::new(HashMap::new()));
    let allow_cidrs = Arc::new(RwLock::new(params.allow_cidrs));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut tasks = JoinSet::new();
    let mut rate_limiter = TokenBucket::new(params.max_rate);
    let mut per_ip_rate_limiter = PerIpRateLimiter::new(params.max_rate_per_ip);
    let connection_timeout = Duration::from_secs(params.timeout_seconds);
    let metrics = Arc::new(Metrics::new());
    if let Some(exp) = params.cert_expiry {
        metrics.tls_cert_expiry.store(exp, Ordering::Relaxed);
    }
    let rejection_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REJECTIONS));
    let tls_acceptor = Arc::new(RwLock::new(params.tls_acceptor));

    let ctx = ServerContext {
        semaphore,
        per_ip_tracker,
        max_connections_per_ip: params.max_connections_per_ip,
        connection_timeout,
        shutdown_tx: shutdown_tx.clone(),
        metrics: Arc::clone(&metrics),
        allow_cidrs: Arc::clone(&allow_cidrs),
        rejection_semaphore,
        silent_reject: params.silent_reject,
        hold_open: params.hold_open,
        tls_acceptor: Arc::clone(&tls_acceptor),
    };

    if let Some((admin_listener, addr)) = admin_binding {
        let admin_metrics = Arc::clone(&metrics);
        let admin_shutdown_rx = shutdown_rx.clone();
        let admin_token = params.admin_token.clone();
        tokio::spawn(async move {
            start_admin_server(
                admin_listener,
                addr,
                admin_metrics,
                admin_token,
                admin_shutdown_rx,
            )
            .await;
        });
    }

    info!("====================================");
    info!("          PORTPROBE STARTED");
    info!("====================================");
    if params.ports.len() == 1 {
        info!("Listening on {}:{}", params.bind_ip, params.ports[0]);
    } else {
        info!(
            "Listening on {} across {} ports: {:?}",
            params.bind_ip,
            params.ports.len(),
            params.ports
        );
    }
    info!("Connection timeout: {} seconds", params.timeout_seconds);
    info!("Maximum connections: {}", params.max_connections);
    info!(
        "Maximum connections per IP: {}",
        params.max_connections_per_ip
    );
    info!("Maximum global rate: {}/second", params.max_rate);
    info!("Maximum per-IP rate: {}/second", params.max_rate_per_ip);
    {
        let cidrs = ctx.allow_cidrs.read().await;
        if !cidrs.is_empty() {
            info!("CIDR allowlist: {} subnets configured", cidrs.len());
        }
    }
    {
        let tls = ctx.tls_acceptor.read().await;
        if tls.is_some() {
            info!("Transport encryption: TLS 1.3 active");
        }
    }
    info!("Silent rejection: {}", ctx.silent_reject);
    info!("Hold open mode: {}", ctx.hold_open);

    info!("Waiting for incoming connections...");
    info!("Press CTRL+C to stop");
    info!("====================================");

    let shutdown_signal = async {
        if let Some(mut rx) = external_shutdown_rx {
            tokio::select! {
                _ = wait_for_shutdown_signal() => {},
                res = rx.changed() => {
                    if res.is_ok() && *rx.borrow() {
                        info!("External shutdown signal received; initiating graceful shutdown...");
                    }
                }
            }
        } else {
            wait_for_shutdown_signal().await;
        }
    };
    tokio::pin!(shutdown_signal);

    let mut metrics_interval = tokio::time::interval(Duration::from_secs(3600));
    metrics_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    metrics_interval.tick().await;

    #[cfg(unix)]
    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).ok();

    // Spawn accept tasks across all listening sockets
    let (conn_tx, mut conn_rx) =
        tokio::sync::mpsc::channel::<(TcpStream, SocketAddr)>(params.max_connections);
    for (listener, addr) in listeners {
        let conn_tx = conn_tx.clone();
        let mut shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, remote_addr)) => {
                                if conn_tx.send((stream, remote_addr)).await.is_err() {
                                    break;
                                }
                            }
                            Err(err) => {
                                error!("Failed to accept connection on {}: {}", addr, err);
                                sleep(ACCEPT_ERROR_BACKOFF).await;
                            }
                        }
                    }
                    res = shutdown_rx.changed() => {
                        if res.is_ok() && *shutdown_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        });
    }
    drop(conn_tx);

    loop {
        select! {
            Some((stream, remote_addr)) = conn_rx.recv() => {
                handle_new_connection(
                    stream,
                    remote_addr,
                    &mut per_ip_rate_limiter,
                    &mut rate_limiter,
                    &ctx,
                    &mut tasks,
                )
                .await;
            }

            () = &mut shutdown_signal => {
                break;
            }

            Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                if let Err(err) = result {
                    error!("Connection task failed: {}", err);
                }
            }

            _ = metrics_interval.tick() => {
                let accepted = metrics.accepted.load(Ordering::Relaxed);
                let rejected = metrics.cidr_rejected.load(Ordering::Relaxed)
                    + metrics.rate_limited.load(Ordering::Relaxed)
                    + metrics.per_ip_rate_limited.load(Ordering::Relaxed)
                    + metrics.connection_limited.load(Ordering::Relaxed)
                    + metrics.per_ip_limited.load(Ordering::Relaxed);
                if accepted > 0 || rejected > 0 {
                    info!(
                        "Periodic metrics checkpoint: {} accepted, {} rejected connections",
                        accepted, rejected
                    );
                }
            }

            _ = async {
                #[cfg(unix)]
                if let Some(ref mut s) = sighup {
                    s.recv().await
                } else {
                    std::future::pending().await
                }
                #[cfg(not(unix))]
                std::future::pending::<Option<()>>().await
            } => {
                info!("SIGHUP signal received; triggering dynamic configuration reload...");
                if let Some(ref cfg_path) = params.config_path {
                    match crate::cli::ConfigFile::load_from_path(cfg_path) {
                        Ok(new_cfg) => {
                            // Reload CIDR subnets
                            if let Some(new_cidrs_str) = new_cfg.allow_cidr {
                                let mut parsed = Vec::new();
                                let mut valid = true;
                                for s in &new_cidrs_str {
                                    match crate::cli::Cidr::parse(s) {
                                        Ok(c) => parsed.push(c),
                                        Err(e) => {
                                            error!("SIGHUP: Failed to parse CIDR '{}': {}", s, e);
                                            valid = false;
                                            break;
                                        }
                                    }
                                }
                                if valid {
                                    let mut lock = ctx.allow_cidrs.write().await;
                                    *lock = parsed;
                                    info!(
                                        "SIGHUP: Successfully reloaded {} CIDR subnet rules from {}",
                                        lock.len(),
                                        cfg_path.display()
                                    );
                                }
                            }

                            // Hot-reload TLS certificates and private key
                            let cert_path_opt = new_cfg.tls_cert.as_ref().or(params.tls_cert_path.as_ref());
                            let key_path_opt = new_cfg.tls_key.as_ref().or(params.tls_key_path.as_ref());
                            let ca_path_opt = new_cfg.tls_client_ca.as_ref().or(params.tls_client_ca_path.as_ref());

                            if let (Some(cert_p), Some(key_p)) = (cert_path_opt, key_path_opt) {
                                match load_tls_acceptor(
                                    cert_p,
                                    key_p,
                                    ca_path_opt.map(|p| p.as_path()),
                                    params.tls_min_version.as_deref(),
                                ) {
                                    Ok((new_acceptor, new_expiry)) => {
                                        let mut lock = ctx.tls_acceptor.write().await;
                                        *lock = Some(new_acceptor);
                                        if let Some(exp) = new_expiry {
                                            ctx.metrics.tls_cert_expiry.store(exp, Ordering::Relaxed);
                                        }
                                        info!(
                                            "SIGHUP: Successfully reloaded TLS certificates and private key from {}",
                                            cert_p.display()
                                        );
                                    }
                                    Err(err) => {
                                        error!(
                                            "SIGHUP: Failed to reload TLS certificates from {}: {}; retaining active certificates",
                                            cert_p.display(),
                                            err
                                        );
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!(
                                "SIGHUP: Failed to reload configuration {}: {}; retaining active settings",
                                cfg_path.display(),
                                err
                            );
                        }
                    }
                } else {
                    info!("SIGHUP: No configuration file specified at startup; reload skipped");
                }
            }
        }
    }

    info!("Stopping listeners...");
    let _ = shutdown_tx.send(true);

    info!("Waiting for active connections to close...");
    while let Some(result) = tasks.join_next().await {
        if let Err(err) = result {
            error!("Connection task failed during shutdown: {}", err);
        }
    }

    info!("All active connections closed.");
    metrics.log_summary();
    info!("PortProbe stopped successfully.");

    Ok(())
}

// ---------------------------------------------------------------------------
// Admission Control Pipeline
// ---------------------------------------------------------------------------

pub async fn handle_new_connection(
    stream: TcpStream,
    remote_addr: SocketAddr,
    per_ip_rate_limiter: &mut PerIpRateLimiter,
    rate_limiter: &mut TokenBucket,
    ctx: &ServerContext,
    tasks: &mut JoinSet<()>,
) {
    let ip = canonicalize_ip(remote_addr.ip());

    // Ingress CIDR evaluation under read lock
    {
        let allow_cidrs = ctx.allow_cidrs.read().await;
        if !allow_cidrs.is_empty() && !allow_cidrs.iter().any(|cidr| cidr.contains(&ip)) {
            ctx.metrics.cidr_rejected.fetch_add(1, Ordering::Relaxed);
            debug!(
                "Connection rejected by CIDR filter: {} is outside allowed subnets. Client: {}",
                ip, remote_addr
            );
            if ctx.silent_reject {
                drop(stream);
            } else {
                spawn_rejection(
                    stream,
                    FORBIDDEN_RESPONSE,
                    Arc::clone(&ctx.rejection_semaphore),
                );
            }
            return;
        }
    }

    // Per-IP rate limiting to prevent single-source token starvation
    if !per_ip_rate_limiter.try_acquire(&ip) {
        ctx.metrics
            .per_ip_rate_limited
            .fetch_add(1, Ordering::Relaxed);
        debug!(
            "Per-IP rate limit reached for {}. Rejecting client: {}",
            ip, remote_addr
        );
        if ctx.silent_reject {
            drop(stream);
        } else {
            spawn_rejection(
                stream,
                RATE_LIMITED_RESPONSE,
                Arc::clone(&ctx.rejection_semaphore),
            );
        }
        return;
    }

    // Global rate limiting
    if !rate_limiter.try_acquire() {
        ctx.metrics.rate_limited.fetch_add(1, Ordering::Relaxed);
        debug!("Connection rate limited. Rejecting client: {}", remote_addr);
        if ctx.silent_reject {
            drop(stream);
        } else {
            spawn_rejection(
                stream,
                RATE_LIMITED_RESPONSE,
                Arc::clone(&ctx.rejection_semaphore),
            );
        }
        return;
    }

    let permit = match ctx.semaphore.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            ctx.metrics
                .connection_limited
                .fetch_add(1, Ordering::Relaxed);
            debug!(
                "Maximum concurrent connections reached. Rejecting client: {}",
                remote_addr
            );
            if ctx.silent_reject {
                drop(stream);
            } else {
                spawn_rejection(
                    stream,
                    CONNECTION_LIMIT_RESPONSE,
                    Arc::clone(&ctx.rejection_semaphore),
                );
            }
            return;
        }
    };

    let (per_ip_exceeded, ip_guard) = {
        let mut counts = ctx
            .per_ip_tracker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let count = counts.entry(ip).or_insert(0);

        if *count >= ctx.max_connections_per_ip {
            (true, None)
        } else {
            *count += 1;
            (
                false,
                Some(PerIpGuard {
                    ip,
                    tracker: Arc::clone(&ctx.per_ip_tracker),
                }),
            )
        }
    };

    if per_ip_exceeded {
        ctx.metrics.per_ip_limited.fetch_add(1, Ordering::Relaxed);
        debug!(
            "Per-IP connection limit ({}) reached for {}. Rejecting client: {}",
            ctx.max_connections_per_ip, ip, remote_addr
        );
        if ctx.silent_reject {
            drop(stream);
        } else {
            spawn_rejection(
                stream,
                PER_IP_LIMIT_RESPONSE,
                Arc::clone(&ctx.rejection_semaphore),
            );
        }
        return;
    }

    let Some(ip_guard) = ip_guard else {
        return;
    };

    let _ = stream.set_nodelay(true);
    let sock_ref = socket2::SockRef::from(&stream);
    let mut ka = socket2::TcpKeepalive::new();
    ka = ka
        .with_time(Duration::from_secs(30))
        .with_interval(Duration::from_secs(10));
    let _ = sock_ref.set_tcp_keepalive(&ka);

    ctx.metrics.accepted.fetch_add(1, Ordering::Relaxed);
    info!("Connection accepted from {}", remote_addr);

    let shutdown_rx = ctx.shutdown_tx.subscribe();
    let deadline = Instant::now() + ctx.connection_timeout;
    let metrics_clone = Arc::clone(&ctx.metrics);
    let tls_acceptor = ctx.tls_acceptor.read().await.clone();
    let hold_open = ctx.hold_open;

    tasks.spawn(async move {
        let _permit = permit;
        let _ip_guard = ip_guard;

        let conn_stream = match tls_acceptor {
            Some(acceptor) => {
                let handshake = timeout_at(
                    Instant::now() + GREETING_WRITE_TIMEOUT,
                    acceptor.accept(stream),
                )
                .await;
                match handshake {
                    Ok(Ok(tls_stream)) => ConnectionStream::TlsServer(Box::new(tls_stream)),
                    Ok(Err(err)) => {
                        // Elevated to warn level for SIEM visibility
                        warn!("TLS handshake failed from {}: {}", remote_addr, err);
                        metrics_clone.errored.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                    Err(_) => {
                        // Elevated to warn level for SIEM visibility
                        warn!("TLS handshake timed out from {}", remote_addr);
                        metrics_clone.timed_out.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                }
            }
            None => ConnectionStream::Plain(stream),
        };

        match handle_connection(conn_stream, remote_addr, deadline, shutdown_rx, hold_open).await {
            Ok(()) => {
                metrics_clone.completed.fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                if err.to_string().contains("timed out") {
                    metrics_clone.timed_out.fetch_add(1, Ordering::Relaxed);
                    debug!("Connection timed out from {}: {}", remote_addr, err);
                } else {
                    metrics_clone.errored.fetch_add(1, Ordering::Relaxed);
                    error!("Connection error from {}: {}", remote_addr, err);
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Connection handler
// ---------------------------------------------------------------------------

pub async fn handle_connection(
    mut stream: ConnectionStream,
    remote_addr: SocketAddr,
    deadline: Instant,
    mut shutdown_rx: watch::Receiver<bool>,
    hold_open: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("TCP connection established: {}", remote_addr);

    let greeting_deadline = Instant::now() + GREETING_WRITE_TIMEOUT;
    match timeout_at(greeting_deadline.min(deadline), stream.write_all(RESPONSE)).await {
        Ok(Ok(())) => {
            info!("Success response sent to {}", remote_addr);
        }
        Ok(Err(ref err))
            if err.kind() == ErrorKind::NotConnected
                || err.kind() == ErrorKind::BrokenPipe
                || err.kind() == ErrorKind::ConnectionReset =>
        {
            debug!(
                "Peer disconnected before response fully delivered: {} ({})",
                remote_addr, err
            );
            return Ok(());
        }
        Ok(Err(err)) => {
            return Err(Box::new(err));
        }
        Err(_) => {
            info!(
                "Greeting write timeout reached while sending response: {}",
                remote_addr
            );
            return Err("Greeting write timed out".into());
        }
    }

    // If hold_open is false (default probe mode), close socket immediately
    if !hold_open {
        let _ = stream.shutdown().await;
        debug!(
            "Connection closed immediately post-response (hold-open disabled): {}",
            remote_addr
        );
        return Ok(());
    }

    let mut drain_buf = [0u8; 2048];
    let mut total_drained: usize = 0;
    loop {
        select! {
            _ = sleep_until(deadline) => {
                info!("Connection timeout reached: {}", remote_addr);
                break;
            }

            read_res = stream.read(&mut drain_buf) => {
                match read_res {
                    Ok(0) => {
                        debug!("Peer disconnected (EOF detected): {}", remote_addr);
                        break;
                    }
                    Ok(n) => {
                        total_drained = total_drained.saturating_add(n);
                        debug!(
                            "Ignored {} unexpected bytes from client (total drained: {}): {}",
                            n, total_drained, remote_addr
                        );
                        if total_drained >= MAX_HOLD_OPEN_DRAIN_BYTES {
                            debug!(
                                "Hold-open payload drain cap reached ({} bytes); terminating connection from {} to prevent slot pinning",
                                total_drained, remote_addr
                            );
                            break;
                        }
                    }
                    Err(err) => {
                        debug!("Peer connection error during hold-open: {} ({})", remote_addr, err);
                        break;
                    }
                }
            }

            result = shutdown_rx.changed() => {
                match result {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!("Closing connection due to server shutdown: {}", remote_addr);
                        break;
                    }
                    _ => {
                        break;
                    }
                }
            }
        }
    }

    match stream.shutdown().await {
        Ok(()) => {}
        Err(ref err)
            if err.kind() == ErrorKind::NotConnected
                || err.kind() == ErrorKind::BrokenPipe
                || err.kind() == ErrorKind::ConnectionReset =>
        {
            debug!(
                "Peer already disconnected during shutdown: {} ({})",
                remote_addr, err
            );
        }
        Err(err) => {
            return Err(Box::new(err));
        }
    }

    info!("Connection closed successfully: {}", remote_addr);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_allows_up_to_rate() {
        let mut bucket = TokenBucket::new(5);
        for i in 0..5 {
            assert!(
                bucket.try_acquire(),
                "connection {} should be allowed",
                i + 1
            );
        }
        assert!(
            !bucket.try_acquire(),
            "over-rate connection should be rejected"
        );
    }

    #[test]
    fn test_token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(10);
        for _ in 0..10 {
            assert!(bucket.try_acquire());
        }
        assert!(!bucket.try_acquire(), "bucket should be empty");
        bucket.last_refill = Instant::now() - Duration::from_secs(1);
        assert!(bucket.try_acquire(), "should allow after refill");
    }

    #[test]
    fn test_token_bucket_caps_at_max() {
        let mut bucket = TokenBucket::new(5);
        bucket.last_refill = Instant::now() - Duration::from_secs(10);

        for i in 0..5 {
            assert!(
                bucket.try_acquire(),
                "connection {} should be allowed",
                i + 1
            );
        }
        assert!(!bucket.try_acquire(), "should not exceed max_tokens");
    }

    #[test]
    fn test_token_bucket_partial_refill() {
        let mut bucket = TokenBucket::new(10);
        for _ in 0..10 {
            bucket.try_acquire();
        }

        bucket.last_refill = Instant::now() - Duration::from_millis(500);

        let mut allowed = 0;
        for _ in 0..10 {
            if bucket.try_acquire() {
                allowed += 1;
            }
        }
        assert!(
            (4..=6).contains(&allowed),
            "expected ~5 tokens after 0.5s, got {}",
            allowed
        );
    }

    #[test]
    fn test_per_ip_guard_decrements_on_drop() {
        let tracker: PerIpTracker = Arc::new(Mutex::new(HashMap::new()));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        {
            let mut counts = tracker.lock().unwrap();
            *counts.entry(ip).or_insert(0) += 1;
        }

        {
            let _guard = PerIpGuard {
                ip,
                tracker: Arc::clone(&tracker),
            };
            assert_eq!(*tracker.lock().unwrap().get(&ip).unwrap(), 1);
        }
        assert!(!tracker.lock().unwrap().contains_key(&ip));
    }

    #[test]
    fn test_per_ip_guard_multiple_connections() {
        let tracker: PerIpTracker = Arc::new(Mutex::new(HashMap::new()));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        {
            let mut counts = tracker.lock().unwrap();
            *counts.entry(ip).or_insert(0) += 2;
        }

        {
            let _guard = PerIpGuard {
                ip,
                tracker: Arc::clone(&tracker),
            };
        }
        assert_eq!(*tracker.lock().unwrap().get(&ip).unwrap(), 1);

        {
            let _guard = PerIpGuard {
                ip,
                tracker: Arc::clone(&tracker),
            };
        }
        assert!(!tracker.lock().unwrap().contains_key(&ip));
    }

    #[test]
    fn test_per_ip_guard_different_ips_independent() {
        let tracker: PerIpTracker = Arc::new(Mutex::new(HashMap::new()));
        let ip_a: IpAddr = "10.0.0.1".parse().unwrap();
        let ip_b: IpAddr = "10.0.0.2".parse().unwrap();

        {
            let mut counts = tracker.lock().unwrap();
            *counts.entry(ip_a).or_insert(0) += 1;
            *counts.entry(ip_b).or_insert(0) += 1;
        }

        {
            let _guard = PerIpGuard {
                ip: ip_a,
                tracker: Arc::clone(&tracker),
            };
        }
        assert!(!tracker.lock().unwrap().contains_key(&ip_a));
        assert!(tracker.lock().unwrap().contains_key(&ip_b));
    }

    #[test]
    fn test_metrics_initial_values() {
        let m = Metrics::new();
        assert_eq!(m.accepted.load(Ordering::Relaxed), 0);
        assert_eq!(m.rate_limited.load(Ordering::Relaxed), 0);
        assert_eq!(m.connection_limited.load(Ordering::Relaxed), 0);
        assert_eq!(m.per_ip_limited.load(Ordering::Relaxed), 0);
        assert_eq!(m.completed.load(Ordering::Relaxed), 0);
        assert_eq!(m.errored.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_metrics_atomic_increments() {
        let m = Metrics::new();
        m.accepted.fetch_add(5, Ordering::Relaxed);
        m.rate_limited.fetch_add(3, Ordering::Relaxed);
        m.completed.fetch_add(4, Ordering::Relaxed);
        m.errored.fetch_add(1, Ordering::Relaxed);

        assert_eq!(m.accepted.load(Ordering::Relaxed), 5);
        assert_eq!(m.rate_limited.load(Ordering::Relaxed), 3);
        assert_eq!(m.completed.load(Ordering::Relaxed), 4);
        assert_eq!(m.errored.load(Ordering::Relaxed), 1);

        let prom = m.format_prometheus();
        assert!(prom.contains("portprobe_accepted_total 5"));
        assert!(prom.contains("portprobe_rate_limited_total 3"));
    }

    #[tokio::test]
    async fn test_peer_disconnect_detected_immediately_during_hold_open() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert!(n > 0);
            assert_eq!(&buf[..n], RESPONSE);
            drop(stream);
        });

        let (server_stream, remote_addr) = listener.accept().await.unwrap();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let deadline = Instant::now() + Duration::from_secs(60);
        let start = Instant::now();

        let res = handle_connection(
            ConnectionStream::Plain(server_stream),
            remote_addr,
            deadline,
            shutdown_rx,
            true, // hold_open enabled
        )
        .await;
        assert!(res.is_ok());

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "expected immediate exit on client disconnect, took {:?}",
            elapsed
        );

        client_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_immediate_disconnect_when_hold_open_is_false() {
        // When hold_open is false, connection closes immediately after banner transmission.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], RESPONSE);
            // Wait for EOF from server shutdown
            let n2 = stream.read(&mut buf).await.unwrap();
            assert_eq!(n2, 0, "server should have closed stream immediately");
        });

        let (server_stream, remote_addr) = listener.accept().await.unwrap();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let deadline = Instant::now() + Duration::from_secs(60);
        let start = Instant::now();

        let res = handle_connection(
            ConnectionStream::Plain(server_stream),
            remote_addr,
            deadline,
            shutdown_rx,
            false, // hold_open disabled (default)
        )
        .await;
        assert!(res.is_ok());

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "expected immediate exit when hold_open is false, took {:?}",
            elapsed
        );

        client_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_spawn_rejection_delivers_banner_and_closes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.unwrap();
            buf
        });

        let (server_stream, _) = listener.accept().await.unwrap();
        let rejection_sem = Arc::new(Semaphore::new(32));
        spawn_rejection(server_stream, RATE_LIMITED_RESPONSE, rejection_sem);

        let received = client_task.await.unwrap();
        assert_eq!(received.as_slice(), RATE_LIMITED_RESPONSE);
    }

    #[test]
    fn test_canonicalize_ip_maps_v4_in_v6() {
        let mapped_v6: IpAddr = "::ffff:192.0.2.1".parse().unwrap();
        let native_v4: IpAddr = "192.0.2.1".parse().unwrap();
        let native_v6: IpAddr = "2001:db8::1".parse().unwrap();

        assert_eq!(canonicalize_ip(mapped_v6), native_v4);
        assert_eq!(canonicalize_ip(native_v4), native_v4);
        assert_eq!(canonicalize_ip(native_v6), native_v6);
    }

    #[test]
    fn test_per_ip_guard_recovers_from_poisoned_mutex() {
        let tracker: PerIpTracker = Arc::new(Mutex::new(HashMap::new()));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        {
            let mut counts = tracker.lock().unwrap();
            counts.insert(ip, 1);
        }

        let tracker_clone = Arc::clone(&tracker);
        let _ = std::thread::spawn(move || {
            let _lock = tracker_clone.lock().unwrap();
            panic!("intentional panic to poison mutex");
        })
        .join();

        assert!(tracker.is_poisoned());

        {
            let _guard = PerIpGuard {
                ip,
                tracker: Arc::clone(&tracker),
            };
        }

        let counts = tracker.lock().unwrap_or_else(|p| p.into_inner());
        assert!(!counts.contains_key(&ip));
    }

    #[tokio::test]
    async fn test_cidr_allowlist_filtering() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.unwrap();
            buf
        });

        let (server_stream, remote_addr) = listener.accept().await.unwrap();
        let semaphore = Arc::new(Semaphore::new(100));
        let per_ip_tracker: PerIpTracker = Arc::new(Mutex::new(HashMap::new()));
        let mut rate_limiter = TokenBucket::new(20);
        let (shutdown_tx, _) = watch::channel(false);
        let mut tasks = JoinSet::new();
        let metrics = Arc::new(Metrics::new());

        let allow_cidrs = Arc::new(RwLock::new(vec![Cidr::parse("10.0.0.0/8").unwrap()]));
        let rejection_semaphore = Arc::new(Semaphore::new(32));

        let ctx = ServerContext {
            semaphore,
            per_ip_tracker,
            max_connections_per_ip: 10,
            connection_timeout: Duration::from_secs(15),
            shutdown_tx,
            metrics: Arc::clone(&metrics),
            allow_cidrs,
            rejection_semaphore,
            silent_reject: false,
            hold_open: false,
            tls_acceptor: Arc::new(RwLock::new(None)),
        };

        let mut per_ip_rate_limiter = PerIpRateLimiter::new(10);
        handle_new_connection(
            server_stream,
            remote_addr,
            &mut per_ip_rate_limiter,
            &mut rate_limiter,
            &ctx,
            &mut tasks,
        )
        .await;

        let received = client_task.await.unwrap();
        assert_eq!(received.as_slice(), FORBIDDEN_RESPONSE);
        assert_eq!(metrics.cidr_rejected.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_hold_open_ignores_unexpected_client_data_until_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert!(n > 0);
            assert_eq!(&buf[..n], RESPONSE);

            stream.write_all(b"GET / HTTP/1.1\r\n\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_millis(150)).await;
            drop(stream);
        });

        let (server_stream, remote_addr) = listener.accept().await.unwrap();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let deadline = Instant::now() + Duration::from_millis(100);
        let start = Instant::now();

        let res = handle_connection(
            ConnectionStream::Plain(server_stream),
            remote_addr,
            deadline,
            shutdown_rx,
            true, // hold_open enabled
        )
        .await;
        assert!(res.is_ok());

        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(90),
            "connection terminated prematurely after client sent bytes, took {:?}",
            elapsed
        );

        client_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_rejection_semaphore_bounds_concurrent_tasks() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = Vec::new();
            let _ = stream.read_to_end(&mut buf).await;
            buf
        });

        let (server_stream, _) = listener.accept().await.unwrap();
        let saturated_sem = Arc::new(Semaphore::new(0));
        spawn_rejection(server_stream, RATE_LIMITED_RESPONSE, saturated_sem);

        let received = client_task.await.unwrap();
        assert!(
            received.is_empty(),
            "expected immediate drop when rejection semaphore is saturated"
        );
    }

    #[tokio::test]
    async fn test_silent_reject_drops_without_banner() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = Vec::new();
            let _ = stream.read_to_end(&mut buf).await;
            buf
        });

        let (server_stream, remote_addr) = listener.accept().await.unwrap();
        let semaphore = Arc::new(Semaphore::new(100));
        let per_ip_tracker: PerIpTracker = Arc::new(Mutex::new(HashMap::new()));
        let mut rate_limiter = TokenBucket::new(20);
        let (shutdown_tx, _) = watch::channel(false);
        let mut tasks = JoinSet::new();
        let metrics = Arc::new(Metrics::new());

        let allow_cidrs = Arc::new(RwLock::new(vec![Cidr::parse("10.0.0.0/8").unwrap()]));
        let rejection_semaphore = Arc::new(Semaphore::new(32));

        let ctx = ServerContext {
            semaphore,
            per_ip_tracker,
            max_connections_per_ip: 10,
            connection_timeout: Duration::from_secs(15),
            shutdown_tx,
            metrics: Arc::clone(&metrics),
            allow_cidrs,
            rejection_semaphore,
            silent_reject: true,
            hold_open: false,
            tls_acceptor: Arc::new(RwLock::new(None)),
        };

        let mut per_ip_rate_limiter = PerIpRateLimiter::new(10);
        handle_new_connection(
            server_stream,
            remote_addr,
            &mut per_ip_rate_limiter,
            &mut rate_limiter,
            &ctx,
            &mut tasks,
        )
        .await;

        let received = client_task.await.unwrap();
        assert!(
            received.is_empty(),
            "silent_reject should return empty response without banner"
        );
        assert_eq!(metrics.cidr_rejected.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_hold_open_caps_payload_drain_and_terminates() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert!(n > 0);

            let payload = vec![b'A'; 32 * 1024];
            let _ = stream.write_all(&payload).await;

            tokio::time::sleep(Duration::from_millis(500)).await;
            drop(stream);
        });

        let (server_stream, remote_addr) = listener.accept().await.unwrap();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let deadline = Instant::now() + Duration::from_secs(10);
        let start = Instant::now();

        let res = handle_connection(
            ConnectionStream::Plain(server_stream),
            remote_addr,
            deadline,
            shutdown_rx,
            true, // hold_open enabled
        )
        .await;
        assert!(res.is_ok());

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(1000),
            "expected immediate termination on payload cap, took {:?}",
            elapsed
        );

        client_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_dual_stack_ipv6_listener_accepts_v4() {
        let socket = socket2::Socket::new(
            socket2::Domain::IPV6,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .unwrap();
        let _ = socket.set_only_v6(false);
        #[cfg(not(windows))]
        socket.set_reuse_address(true).unwrap();
        #[cfg(windows)]
        let _ = socket.set_reuse_address(false);
        let bind_addr: SocketAddr = "[::]:0".parse().unwrap();
        socket.bind(&bind_addr.into()).unwrap();
        socket.listen(128).unwrap();
        socket.set_nonblocking(true).unwrap();
        let listener = TcpListener::from_std(socket.into()).unwrap();
        let local_addr = listener.local_addr().unwrap();
        assert!(local_addr.port() > 0);
    }

    #[tokio::test]
    async fn test_post_drain_disconnect_reclaims_slot_immediately() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert!(n > 0);

            let payload = vec![b'Z'; 18 * 1024];
            let _ = stream.write_all(&payload).await;

            drop(stream);
        });

        let (server_stream, remote_addr) = listener.accept().await.unwrap();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let deadline = Instant::now() + Duration::from_secs(60);
        let start = Instant::now();

        let res = handle_connection(
            ConnectionStream::Plain(server_stream),
            remote_addr,
            deadline,
            shutdown_rx,
            true, // hold_open enabled
        )
        .await;
        assert!(res.is_ok());

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(1000),
            "expected immediate slot reclamation on payload cap, took {:?}",
            elapsed
        );

        client_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_admin_server_healthz_and_metrics() {
        let metrics = Arc::new(Metrics::new());
        metrics.accepted.fetch_add(42, Ordering::Relaxed);
        metrics.rate_limited.fetch_add(3, Ordering::Relaxed);

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let test_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = test_listener.local_addr().unwrap();

        tokio::spawn(start_admin_server(
            test_listener,
            addr,
            Arc::clone(&metrics),
            None,
            shutdown_rx,
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Test GET /healthz
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
        assert!(resp_str.contains("OK\n"));

        // Test HEAD /healthz for load balancers
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"HEAD /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
        assert!(resp_str.contains("Content-Length: 3"));

        // Test GET /metrics
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
        assert!(resp_str.contains("portprobe_accepted_total 42"));
        assert!(resp_str.contains("portprobe_rate_limited_total 3"));

        let _ = shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn test_admin_server_timeout_on_idle_connection() {
        // Admin server must not hang indefinitely when an idle client sends 0 bytes.
        let metrics = Arc::new(Metrics::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let test_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = test_listener.local_addr().unwrap();

        tokio::spawn(start_admin_server(
            test_listener,
            addr,
            Arc::clone(&metrics),
            None,
            shutdown_rx,
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;

        let start = Instant::now();
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 16];
        // Read until EOF without sending any bytes to trigger read timeout
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(n, 0, "admin server should terminate idle connection");
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(900) && elapsed < Duration::from_millis(2500),
            "expected idle timeout around 1s, got {:?}",
            elapsed
        );

        let _ = shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn test_client_health_check_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let _ = stream.write_all(RESPONSE).await;
            }
        });

        let check_res = run_health_check(addr, None).await;
        assert!(check_res.is_ok());
    }

    #[tokio::test]
    async fn test_client_health_check_fails_on_rate_limit_banner() {
        // Healthcheck must FAIL if the server returns RATE_LIMITED_RESPONSE or error banners
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let _ = stream.write_all(RATE_LIMITED_RESPONSE).await;
            }
        });

        let check_res = run_health_check(addr, None).await;
        assert!(
            check_res.is_err(),
            "healthcheck must fail when server returns rate limit banner"
        );
        let err_str = check_res.unwrap_err().to_string();
        assert!(err_str.contains("unexpected banner") || err_str.contains("rejected"));
    }

    #[tokio::test]
    async fn test_client_health_check_fails_on_cidr_forbidden_banner() {
        // Healthcheck must FAIL if the probe is blocked by CIDR filter
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let _ = stream.write_all(FORBIDDEN_RESPONSE).await;
            }
        });

        let check_res = run_health_check(addr, None).await;
        assert!(
            check_res.is_err(),
            "healthcheck must fail when server returns CIDR forbidden banner"
        );
    }

    #[tokio::test]
    async fn test_unified_ipv4_socket_creation() {
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .unwrap();
        #[cfg(not(windows))]
        socket.set_reuse_address(true).unwrap();
        #[cfg(windows)]
        let _ = socket.set_reuse_address(false);

        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        socket.bind(&bind_addr.into()).unwrap();
        socket.listen(128).unwrap();
        socket.set_nonblocking(true).unwrap();

        let listener = TcpListener::from_std(socket.into()).unwrap();
        assert!(listener.local_addr().unwrap().port() > 0);
    }

    #[tokio::test]
    async fn test_admin_server_bearer_token_rejection() {
        // Admin server must reject requests when --admin-token is set and client provides no token or wrong token.
        let metrics = Arc::new(Metrics::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let test_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = test_listener.local_addr().unwrap();

        tokio::spawn(start_admin_server(
            test_listener,
            addr,
            Arc::clone(&metrics),
            Some("SecretEnterpriseToken123".to_string()),
            shutdown_rx,
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 1. Request with NO token -> 401 Unauthorized
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.starts_with("HTTP/1.1 401 Unauthorized"));
        assert!(resp_str.contains("WWW-Authenticate: Bearer"));

        // 2. Request with WRONG token -> 401 Unauthorized
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer WrongToken\r\n\r\n")
            .await
            .unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.starts_with("HTTP/1.1 401 Unauthorized"));

        let _ = shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn test_admin_server_bearer_token_success() {
        // Admin server must accept requests when valid Bearer token is provided.
        let metrics = Arc::new(Metrics::new());
        metrics.accepted.fetch_add(99, Ordering::Relaxed);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let test_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = test_listener.local_addr().unwrap();

        tokio::spawn(start_admin_server(
            test_listener,
            addr,
            Arc::clone(&metrics),
            Some("SecretEnterpriseToken123".to_string()),
            shutdown_rx,
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer SecretEnterpriseToken123\r\n\r\n")
            .await
            .unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
        assert!(resp_str.contains("portprobe_accepted_total 99"));

        let _ = shutdown_tx.send(true);
    }

    #[test]
    fn test_per_ip_rate_limiter_isolates_aggressive_ip() {
        // An aggressive IP bursting beyond its per-IP limit is throttled without exhausting other IPs.
        let mut limiter = PerIpRateLimiter::new(2);
        let ip_a: IpAddr = "10.0.0.1".parse().unwrap();
        let ip_b: IpAddr = "10.0.0.2".parse().unwrap();

        // IP A consumes its 2 tokens
        assert!(limiter.try_acquire(&ip_a));
        assert!(limiter.try_acquire(&ip_a));
        // IP A 3rd attempt is rejected
        assert!(!limiter.try_acquire(&ip_a));

        // IP B can still acquire its tokens (NOT starved by IP A)
        assert!(limiter.try_acquire(&ip_b));
        assert!(limiter.try_acquire(&ip_b));
        assert!(!limiter.try_acquire(&ip_b));
    }

    #[tokio::test]
    async fn test_multi_port_listeners_bind_and_accept() {
        // Server can bind and accept connections across multiple ports.
        let (conn_tx, mut conn_rx) = tokio::sync::mpsc::channel::<(TcpStream, SocketAddr)>(10);
        let (shutdown_tx, _) = watch::channel(false);

        let listener1 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr1 = listener1.local_addr().unwrap();
        let listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr2 = listener2.local_addr().unwrap();

        for listener in [listener1, listener2] {
            let conn_tx = conn_tx.clone();
            let mut shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        res = listener.accept() => {
                            if let Ok((stream, remote_addr)) = res {
                                if conn_tx.send((stream, remote_addr)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        _ = shutdown_rx.changed() => break,
                    }
                }
            });
        }
        drop(conn_tx);

        // Connect to port 1
        let _c1 = TcpStream::connect(addr1).await.unwrap();
        let res1 = conn_rx.recv().await;
        assert!(res1.is_some());

        // Connect to port 2
        let _c2 = TcpStream::connect(addr2).await.unwrap();
        let res2 = conn_rx.recv().await;
        assert!(res2.is_some());

        let _ = shutdown_tx.send(true);
    }

    #[test]
    fn test_constant_time_token_verify_correctness() {
        // Constant-time token verification handles matching, mismatched, different-length, and empty tokens
        assert!(constant_time_token_verify("Secret123", "Secret123"));
        assert!(!constant_time_token_verify("Secret124", "Secret123"));
        assert!(!constant_time_token_verify("Secret", "Secret123"));
        assert!(!constant_time_token_verify("Secret12345", "Secret123"));
        assert!(!constant_time_token_verify("", "Secret123"));
        assert!(!constant_time_token_verify("Secret123", ""));
        assert!(constant_time_token_verify("", ""));
    }

    #[test]
    fn test_rate_limit_key_ipv6_subnet_aggregation() {
        // Masking IPv6 addresses to /64 routing prefix
        let host1: IpAddr = "2001:db8:abcd:0012:0000:0000:0000:0001".parse().unwrap();
        let host2: IpAddr = "2001:db8:abcd:0012:ffff:ffff:ffff:ffff".parse().unwrap();
        let host_diff_subnet: IpAddr = "2001:db8:abcd:0013:0000:0000:0000:0001".parse().unwrap();

        assert_eq!(rate_limit_key(host1), rate_limit_key(host2));
        assert_ne!(rate_limit_key(host1), rate_limit_key(host_diff_subnet));

        let v4: IpAddr = "192.168.1.100".parse().unwrap();
        assert_eq!(rate_limit_key(v4), v4);
    }

    #[tokio::test]
    async fn test_admin_server_header_fragmentation_reassembly() {
        // TCP fragmentation across packets is properly buffered and reassembled
        let metrics = Arc::new(Metrics::new());
        metrics.accepted.fetch_add(77, Ordering::Relaxed);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let test_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = test_listener.local_addr().unwrap();

        tokio::spawn(start_admin_server(
            test_listener,
            addr,
            Arc::clone(&metrics),
            None,
            shutdown_rx,
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();
        // Send first fragment
        stream.write_all(b"GET /met").await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Send second fragment with header terminator
        stream
            .write_all(b"rics HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();

        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(
            resp_str.starts_with("HTTP/1.1 200 OK"),
            "Expected 200 OK on fragmented headers, got: {}",
            resp_str
        );
        assert!(resp_str.contains("portprobe_accepted_total 77"));

        let _ = shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn test_admin_server_header_size_limit_rejection() {
        // Headers exceeding 4096 bytes are rejected with 431 Request Header Fields Too Large
        let metrics = Arc::new(Metrics::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let test_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = test_listener.local_addr().unwrap();

        tokio::spawn(start_admin_server(
            test_listener,
            addr,
            Arc::clone(&metrics),
            None,
            shutdown_rx,
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let big_payload = vec![b'X'; 4100];
        let _ = stream.write_all(&big_payload).await;
        let _ = stream.shutdown().await;

        let mut resp = Vec::new();
        let _ = stream.read_to_end(&mut resp).await;
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(
            resp_str.starts_with("HTTP/1.1 431"),
            "Expected 431 on header overflow, got: {}",
            resp_str
        );

        let _ = shutdown_tx.send(true);
    }

    #[test]
    fn test_metrics_cert_expiry_prometheus_output() {
        let metrics = Metrics::new();
        assert!(!metrics
            .format_prometheus()
            .contains("portprobe_tls_cert_expiry_timestamp_seconds"));

        metrics.tls_cert_expiry.store(1763553600, Ordering::Relaxed);
        let prom = metrics.format_prometheus();
        assert!(prom.contains("portprobe_tls_cert_expiry_timestamp_seconds 1763553600"));
    }
}
