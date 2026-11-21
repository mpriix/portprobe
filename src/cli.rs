//! CLI argument parsing and validation for PortProbe.
//!
//! Uses `clap` with derive macros for ergonomic argument definitions.
//! Each parameter has a dedicated validation function that enforces
//! safe operational ranges, rejecting dangerous configurations at
//! startup rather than at runtime.

use clap::Parser;
use serde::Deserialize;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use crate::logger::LogFormat;

/// Validate file permissions on POSIX systems to ensure secrets are not accessible by others.
#[cfg(unix)]
pub fn verify_file_permissions<P: AsRef<Path>>(path: P) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?;
    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        return Err(format!(
            "Insecure permissions on {}: {:o}. Must not be accessible by group or others (enforce 0600 or 0400)",
            path.display(),
            mode & 0o777
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn verify_file_permissions<P: AsRef<Path>>(_path: P) -> Result<(), String> {
    Ok(())
}

/// Validate that the timeout is within the allowed range (1–3600 seconds).
fn validate_timeout(value: &str) -> Result<u64, String> {
    let timeout: u64 = value
        .parse()
        .map_err(|_| "Timeout must be a valid number".to_string())?;

    if !(1..=3600).contains(&timeout) {
        return Err("Timeout must be between 1 and 3600 seconds".to_string());
    }

    Ok(timeout)
}

/// Validate that max connections is within the allowed range (1–10000).
fn validate_max_connections(value: &str) -> Result<usize, String> {
    let connections: usize = value
        .parse()
        .map_err(|_| "Maximum connections must be a valid number".to_string())?;

    if !(1..=10000).contains(&connections) {
        return Err("Maximum connections must be between 1 and 10000".to_string());
    }

    Ok(connections)
}

/// Validate that the per-IP connection limit is within the allowed range (1–1000).
fn validate_max_connections_per_ip(value: &str) -> Result<usize, String> {
    let connections: usize = value
        .parse()
        .map_err(|_| "Per-IP connection limit must be a valid number".to_string())?;

    if !(1..=1000).contains(&connections) {
        return Err("Per-IP connection limit must be between 1 and 1000".to_string());
    }

    Ok(connections)
}

/// Validate that the connection rate is within the allowed range (1–1000 connections/second).
fn validate_max_rate(value: &str) -> Result<u64, String> {
    let rate: u64 = value
        .parse()
        .map_err(|_| "Maximum rate must be a valid number".to_string())?;

    if !(1..=1000).contains(&rate) {
        return Err("Maximum rate must be between 1 and 1000".to_string());
    }

    Ok(rate)
}

/// Validate that the per-IP connection rate is within the allowed range (1–1000 connections/second).
fn validate_max_rate_per_ip(value: &str) -> Result<u64, String> {
    let rate: u64 = value
        .parse()
        .map_err(|_| "Per-IP rate limit must be a valid number".to_string())?;

    if !(1..=1000).contains(&rate) {
        return Err("Per-IP rate limit must be between 1 and 1000".to_string());
    }

    Ok(rate)
}

/// PortProbe CLI arguments.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "portprobe",
    version,
    about = "Secure TCP connectivity testing utility"
)]
pub struct Cli {
    /// TCP port to listen on (1–65535).
    #[arg(short, long, env = "PORTPROBE_PORT")]
    pub port: Option<u16>,

    /// Optional comma-separated list or ranges of TCP ports (e.g. "80,443,8080-8085").
    #[arg(long, value_delimiter = ',', env = "PORTPROBE_PORTS")]
    pub ports: Vec<String>,

    /// IP address to bind to.
    ///
    /// Defaults to 127.0.0.1 (localhost only).
    /// Use 0.0.0.0 to accept connections on all IPv4 interfaces.
    #[arg(short, long, env = "PORTPROBE_BIND")]
    pub bind: Option<IpAddr>,

    /// Connection lifetime in seconds (1–3600).
    #[arg(
        short,
        long,
        value_parser = validate_timeout,
        env = "PORTPROBE_TIMEOUT"
    )]
    pub timeout: Option<u64>,

    /// Maximum simultaneous connections across all clients (1–10000).
    #[arg(
        long,
        value_parser = validate_max_connections,
        env = "PORTPROBE_MAX_CONNECTIONS"
    )]
    pub max_connections: Option<usize>,

    /// Maximum simultaneous connections from a single IP (1–1000).
    #[arg(
        long,
        value_parser = validate_max_connections_per_ip,
        env = "PORTPROBE_MAX_CONNECTIONS_PER_IP"
    )]
    pub max_connections_per_ip: Option<usize>,

    /// Maximum accepted connections per second globally (1–1000).
    #[arg(
        long,
        value_parser = validate_max_rate,
        env = "PORTPROBE_MAX_RATE"
    )]
    pub max_rate: Option<u64>,

    /// Maximum accepted connections per second from a single IP (1–1000).
    #[arg(
        long,
        value_parser = validate_max_rate_per_ip,
        env = "PORTPROBE_MAX_RATE_PER_IP"
    )]
    pub max_rate_per_ip: Option<u64>,

    /// Optional comma-separated list of allowed CIDR subnets (e.g. "10.0.0.0/8,192.168.1.0/24").
    #[arg(long, value_delimiter = ',')]
    pub allow_cidr: Vec<String>,

    /// Silently drop rejected connections without transmitting diagnostic error banners.
    #[arg(
        long,
        env = "PORTPROBE_SILENT_REJECT",
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub silent_reject: Option<bool>,

    /// Keep TCP socket open post-response until timeout expires; false closes immediately.
    #[arg(
        long,
        env = "PORTPROBE_HOLD_OPEN",
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub hold_open: Option<bool>,

    /// Path to a TOML configuration file (e.g. /etc/portprobe/portprobe.toml).
    #[arg(short = 'c', long, env = "PORTPROBE_CONFIG")]
    pub config: Option<PathBuf>,

    /// Log output format: 'text' (default) or 'json' for SIEM ingestion.
    #[arg(long, env = "PORTPROBE_LOG_FORMAT")]
    pub log_format: Option<LogFormat>,

    /// Execute client healthcheck against the configured host and port, exiting 0 on success or 1 on failure.
    #[arg(long, default_value_t = false)]
    pub health_check: bool,

    /// Connect with TLS encryption during client healthcheck.
    #[arg(long, default_value_t = false)]
    pub health_check_tls: bool,

    /// Expected TLS domain name / SNI during healthcheck (defaults to localhost).
    #[arg(long)]
    pub health_check_domain: Option<String>,

    /// Path to custom CA certificate PEM file to trust for healthcheck TLS validation.
    #[arg(long)]
    pub health_check_ca: Option<PathBuf>,

    /// Optional port for administrative Prometheus metrics and /healthz endpoint on 127.0.0.1.
    #[arg(long, env = "PORTPROBE_ADMIN_PORT")]
    pub admin_port: Option<u16>,

    /// Optional Bearer token for authenticating administrative /metrics and /healthz endpoints.
    /// WARNING: Passing credentials via CLI arguments is deprecated as it exposes them in OS process listings.
    /// Prefer PORTPROBE_ADMIN_TOKEN or admin_token_file.
    #[arg(long, env = "PORTPROBE_ADMIN_TOKEN")]
    pub admin_token: Option<String>,

    /// Path to file containing Bearer token for authenticating administrative endpoints.
    #[arg(long, env = "PORTPROBE_ADMIN_TOKEN_FILE")]
    pub admin_token_file: Option<PathBuf>,

    /// Minimum TLS protocol version: "1.2" (default) or "1.3" (strict banking compliance).
    #[arg(long, env = "PORTPROBE_TLS_MIN_VERSION")]
    pub tls_min_version: Option<String>,

    /// Path to TLS server certificate chain PEM file for encrypted transport.
    #[arg(long, env = "PORTPROBE_TLS_CERT")]
    pub tls_cert: Option<PathBuf>,

    /// Path to TLS server private key PEM file.
    #[arg(long, env = "PORTPROBE_TLS_KEY")]
    pub tls_key: Option<PathBuf>,

    /// Path to optional CA certificate PEM bundle for mutual TLS (mTLS) client verification.
    #[arg(long, env = "PORTPROBE_TLS_CLIENT_CA")]
    pub tls_client_ca: Option<PathBuf>,

    /// Unprivileged user account to transition to post-socket-bind on Linux/Unix.
    #[arg(long, env = "PORTPROBE_USER")]
    pub user: Option<String>,

    /// Unprivileged group account to transition to post-socket-bind on Linux/Unix.
    #[arg(long, env = "PORTPROBE_GROUP")]
    pub group: Option<String>,

    /// Run as a managed background Windows Service.
    #[arg(long, default_value_t = false)]
    pub service: bool,
}

/// Structured representation of configuration loaded from TOML file.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ConfigFile {
    pub port: Option<u16>,
    pub ports: Option<Vec<String>>,
    pub bind: Option<IpAddr>,
    pub timeout: Option<u64>,
    pub max_connections: Option<usize>,
    pub max_connections_per_ip: Option<usize>,
    pub max_rate: Option<u64>,
    pub max_rate_per_ip: Option<u64>,
    pub allow_cidr: Option<Vec<String>>,
    pub silent_reject: Option<bool>,
    pub hold_open: Option<bool>,
    pub log_format: Option<LogFormat>,
    pub admin_port: Option<u16>,
    pub admin_token: Option<String>,
    pub admin_token_file: Option<PathBuf>,
    pub tls_min_version: Option<String>,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub tls_client_ca: Option<PathBuf>,
    pub user: Option<String>,
    pub group: Option<String>,
}

impl ConfigFile {
    /// Load and parse configuration from a TOML file.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(format!("Configuration file not found: {}", path.display()));
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            format!(
                "Failed to read configuration file {}: {}",
                path.display(),
                e
            )
        })?;
        toml::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse TOML configuration {}: {}",
                path.display(),
                e
            )
        })
    }
}

/// Represents an IPv4 or IPv6 CIDR subnet or host mask for ingress filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cidr {
    V4 { network: u32, mask: u32 },
    V6 { network: u128, mask: u128 },
}

impl Cidr {
    /// Parse a CIDR notation string (e.g. "10.0.0.0/8", "192.168.1.1", or "::ffff:192.168.1.0/120").
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('/').collect();
        match parts.len() {
            1 => {
                let ip_str = parts[0].trim();
                let ip: IpAddr = ip_str
                    .parse()
                    .map_err(|_| format!("Invalid IP address: {}", ip_str))?;
                match ip {
                    IpAddr::V4(v4) => Ok(Cidr::V4 {
                        network: u32::from(v4),
                        mask: !0u32,
                    }),
                    IpAddr::V6(v6) => {
                        if let Some(v4) = v6.to_ipv4_mapped() {
                            Ok(Cidr::V4 {
                                network: u32::from(v4),
                                mask: !0u32,
                            })
                        } else {
                            Ok(Cidr::V6 {
                                network: u128::from(v6),
                                mask: !0u128,
                            })
                        }
                    }
                }
            }
            2 => {
                let ip_str = parts[0].trim();
                let ip: IpAddr = ip_str
                    .parse()
                    .map_err(|_| format!("Invalid IP address: {}", ip_str))?;
                let prefix_str = parts[1].trim();
                let prefix: u8 = prefix_str
                    .parse()
                    .map_err(|_| format!("Invalid prefix length: {}", prefix_str))?;
                match ip {
                    IpAddr::V4(v4) => {
                        if prefix > 32 {
                            return Err(format!(
                                "IPv4 prefix length must be between 0 and 32, got {}",
                                prefix
                            ));
                        }
                        let mask = if prefix == 0 {
                            0u32
                        } else {
                            !0u32 << (32 - prefix)
                        };
                        let network = u32::from(v4) & mask;
                        Ok(Cidr::V4 { network, mask })
                    }
                    IpAddr::V6(v6) => {
                        if prefix > 128 {
                            return Err(format!(
                                "IPv6 prefix length must be between 0 and 128, got {}",
                                prefix
                            ));
                        }
                        if let Some(v4) = v6.to_ipv4_mapped() {
                            if prefix < 96 {
                                return Err(format!(
                                    "IPv4-mapped IPv6 prefix must be >= 96, got {}",
                                    prefix
                                ));
                            }
                            let v4_prefix = prefix - 96;
                            let mask = if v4_prefix == 0 {
                                0u32
                            } else {
                                !0u32 << (32 - v4_prefix)
                            };
                            let network = u32::from(v4) & mask;
                            Ok(Cidr::V4 { network, mask })
                        } else {
                            let mask = if prefix == 0 {
                                0u128
                            } else {
                                !0u128 << (128 - prefix)
                            };
                            let network = u128::from(v6) & mask;
                            Ok(Cidr::V6 { network, mask })
                        }
                    }
                }
            }
            _ => Err(format!("Malformed CIDR notation: {}", s)),
        }
    }

    /// Check whether the given IP address is contained within this CIDR subnet.
    pub fn contains(&self, ip: &IpAddr) -> bool {
        let canonical_ip = match ip {
            IpAddr::V6(v6) => {
                if let Some(v4) = v6.to_ipv4_mapped() {
                    IpAddr::V4(v4)
                } else {
                    IpAddr::V6(*v6)
                }
            }
            IpAddr::V4(v4) => IpAddr::V4(*v4),
        };

        match (self, &canonical_ip) {
            (Cidr::V4 { network, mask }, IpAddr::V4(v4)) => (u32::from(*v4) & mask) == *network,
            (Cidr::V6 { network, mask }, IpAddr::V6(v6)) => (u128::from(*v6) & mask) == *network,
            _ => false,
        }
    }
}

impl Cli {
    /// Merge settings from an optional TOML configuration file into this CLI instance.
    /// Values explicitly passed on the CLI take precedence over configuration file values.
    pub fn apply_config_file(&mut self) -> Result<(), String> {
        if let Some(ref config_path) = self.config {
            let file_cfg = ConfigFile::load_from_path(config_path)?;
            if self.port.is_none() {
                self.port = file_cfg.port;
            }
            if self.ports.is_empty() {
                if let Some(ports) = file_cfg.ports {
                    self.ports = ports;
                }
            }
            if self.bind.is_none() {
                self.bind = file_cfg.bind;
            }
            if self.timeout.is_none() {
                self.timeout = file_cfg.timeout;
            }
            if self.max_connections.is_none() {
                self.max_connections = file_cfg.max_connections;
            }
            if self.max_connections_per_ip.is_none() {
                self.max_connections_per_ip = file_cfg.max_connections_per_ip;
            }
            if self.max_rate.is_none() {
                self.max_rate = file_cfg.max_rate;
            }
            if self.max_rate_per_ip.is_none() {
                self.max_rate_per_ip = file_cfg.max_rate_per_ip;
            }
            if self.allow_cidr.is_empty() {
                if let Some(cidrs) = file_cfg.allow_cidr {
                    self.allow_cidr = cidrs;
                }
            }
            if self.silent_reject.is_none() {
                self.silent_reject = file_cfg.silent_reject;
            }
            if self.hold_open.is_none() {
                self.hold_open = file_cfg.hold_open;
            }
            if self.log_format.is_none() {
                self.log_format = file_cfg.log_format;
            }
            if self.admin_port.is_none() {
                self.admin_port = file_cfg.admin_port;
            }
            if self.admin_token.is_none() {
                self.admin_token = file_cfg.admin_token;
            }
            if self.admin_token_file.is_none() {
                self.admin_token_file = file_cfg.admin_token_file;
            }
            if self.tls_min_version.is_none() {
                self.tls_min_version = file_cfg.tls_min_version;
            }
            if self.tls_cert.is_none() {
                self.tls_cert = file_cfg.tls_cert;
            }
            if self.tls_key.is_none() {
                self.tls_key = file_cfg.tls_key;
            }
            if self.tls_client_ca.is_none() {
                self.tls_client_ca = file_cfg.tls_client_ca;
            }
            if self.user.is_none() {
                self.user = file_cfg.user;
            }
            if self.group.is_none() {
                self.group = file_cfg.group;
            }
        }
        Ok(())
    }

    /// Resolve any unspecified CLI options to secure enterprise defaults.
    pub fn resolve_defaults(&mut self) {
        if self.admin_token.is_none() {
            if let Some(ref path) = self.admin_token_file {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let trimmed = content.trim().to_string();
                    if !trimmed.is_empty() {
                        self.admin_token = Some(trimmed);
                    }
                }
            }
        }
        if self.bind.is_none() {
            self.bind = Some(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
        }
        if self.timeout.is_none() {
            self.timeout = Some(15);
        }
        if self.max_connections.is_none() {
            self.max_connections = Some(100);
        }
        if self.max_connections_per_ip.is_none() {
            self.max_connections_per_ip = Some(10);
        }
        if self.max_rate.is_none() {
            self.max_rate = Some(20);
        }
        if self.max_rate_per_ip.is_none() {
            self.max_rate_per_ip = Some(5); // Default to 5 req/s per IP to prevent global token starvation
        }
        if self.silent_reject.is_none() {
            self.silent_reject = Some(true); // Secure by default: block reconnaissance
        }
        if self.hold_open.is_none() {
            self.hold_open = Some(false); // Secure by default: release slots immediately
        }
    }

    /// Parse and return all configured CIDR subnets.
    pub fn parse_cidrs(&self) -> Result<Vec<Cidr>, String> {
        self.allow_cidr.iter().map(|s| Cidr::parse(s)).collect()
    }

    /// Parse and resolve all configured listening ports, expanding ranges and deduplicating.
    pub fn resolve_all_ports(&self) -> Result<Vec<u16>, String> {
        let mut resolved = Vec::new();

        if let Some(p) = self.port {
            if p == 0 {
                return Err("Port must be between 1 and 65535".to_string());
            }
            resolved.push(p);
        }

        for item in &self.ports {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            if let Some((start_s, end_s)) = item.split_once('-') {
                let start: u16 = start_s
                    .trim()
                    .parse()
                    .map_err(|_| format!("Invalid start port in range: {}", start_s))?;
                let end: u16 = end_s
                    .trim()
                    .parse()
                    .map_err(|_| format!("Invalid end port in range: {}", end_s))?;
                if start == 0 || end == 0 {
                    return Err("Port in range must be between 1 and 65535".to_string());
                }
                if start > end {
                    return Err(format!(
                        "Invalid port range: start ({}) exceeds end ({})",
                        start, end
                    ));
                }
                if (end - start) > 100 {
                    return Err(format!(
                        "Port range too large: {}-{} exceeds maximum range of 100 ports",
                        start, end
                    ));
                }
                for p in start..=end {
                    resolved.push(p);
                }
            } else {
                let p: u16 = item
                    .parse()
                    .map_err(|_| format!("Invalid port number: {}", item))?;
                if p == 0 {
                    return Err("Port must be between 1 and 65535".to_string());
                }
                resolved.push(p);
            }
        }

        resolved.sort_unstable();
        resolved.dedup();

        if resolved.is_empty() {
            return Err("Port must be between 1 and 65535".to_string());
        }

        if resolved.len() > 50 {
            return Err(format!(
                "Total number of listening ports ({}) exceeds maximum limit of 50",
                resolved.len()
            ));
        }

        Ok(resolved)
    }

    /// Validate argument combinations that clap's `value_parser` cannot check individually.
    pub fn validate(&self) -> Result<(), String> {
        self.resolve_all_ports()?;

        let max_conns = self.max_connections.unwrap_or(100);
        let max_per_ip = self.max_connections_per_ip.unwrap_or(10);

        if max_per_ip > max_conns {
            return Err(format!(
                "Per-IP connection limit ({}) cannot exceed global limit ({})",
                max_per_ip, max_conns
            ));
        }

        let max_rate = self.max_rate.unwrap_or(20);
        let max_rate_per_ip = self.max_rate_per_ip.unwrap_or(5);
        if max_rate_per_ip > max_rate {
            return Err(format!(
                "Per-IP rate limit ({}) cannot exceed global rate limit ({})",
                max_rate_per_ip, max_rate
            ));
        }

        if let Some(ref token) = self.admin_token {
            if token.trim().is_empty() {
                return Err("Admin token cannot be empty when configured".to_string());
            }
        }

        if let Some(ref path) = self.admin_token_file {
            if !path.exists() {
                return Err(format!("Admin token file not found: {}", path.display()));
            }
            verify_file_permissions(path)?;
        }

        if let Some(ref path) = self.tls_key {
            if path.exists() {
                verify_file_permissions(path)?;
            }
        }

        if let Some(ref v) = self.tls_min_version {
            if v != "1.2" && v != "1.3" {
                return Err(format!(
                    "Invalid tls_min_version '{}'. Allowed values: '1.2' or '1.3'",
                    v
                ));
            }
        }

        if self.tls_cert.is_some() != self.tls_key.is_some() {
            return Err(
                "Both --tls-cert and --tls-key must be specified together for TLS".to_string(),
            );
        }

        if self.tls_client_ca.is_some() && self.tls_cert.is_none() {
            return Err(
                "--tls-client-ca requires --tls-cert and --tls-key to be specified".to_string(),
            );
        }

        self.parse_cidrs()?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_timeout_lower_bound() {
        assert_eq!(validate_timeout("1"), Ok(1));
    }

    #[test]
    fn test_validate_timeout_typical_value() {
        assert_eq!(validate_timeout("15"), Ok(15));
    }

    #[test]
    fn test_validate_timeout_upper_bound() {
        assert_eq!(validate_timeout("3600"), Ok(3600));
    }

    #[test]
    fn test_validate_timeout_zero_rejected() {
        assert!(validate_timeout("0").is_err());
    }

    #[test]
    fn test_validate_timeout_exceeds_max() {
        assert!(validate_timeout("3601").is_err());
    }

    #[test]
    fn test_validate_timeout_non_numeric() {
        assert!(validate_timeout("abc").is_err());
    }

    #[test]
    fn test_validate_timeout_negative_rejected() {
        assert!(validate_timeout("-1").is_err());
    }

    #[test]
    fn test_validate_max_connections_lower_bound() {
        assert_eq!(validate_max_connections("1"), Ok(1));
    }

    #[test]
    fn test_validate_max_connections_typical_value() {
        assert_eq!(validate_max_connections("100"), Ok(100));
    }

    #[test]
    fn test_validate_max_connections_upper_bound() {
        assert_eq!(validate_max_connections("10000"), Ok(10000));
    }

    #[test]
    fn test_validate_max_connections_zero_rejected() {
        assert!(validate_max_connections("0").is_err());
    }

    #[test]
    fn test_validate_max_connections_exceeds_max() {
        assert!(validate_max_connections("10001").is_err());
    }

    #[test]
    fn test_validate_per_ip_lower_bound() {
        assert_eq!(validate_max_connections_per_ip("1"), Ok(1));
    }

    #[test]
    fn test_validate_per_ip_typical_value() {
        assert_eq!(validate_max_connections_per_ip("10"), Ok(10));
    }

    #[test]
    fn test_validate_per_ip_upper_bound() {
        assert_eq!(validate_max_connections_per_ip("1000"), Ok(1000));
    }

    #[test]
    fn test_validate_per_ip_zero_rejected() {
        assert!(validate_max_connections_per_ip("0").is_err());
    }

    #[test]
    fn test_validate_per_ip_exceeds_max() {
        assert!(validate_max_connections_per_ip("1001").is_err());
    }

    #[test]
    fn test_validate_max_rate_lower_bound() {
        assert_eq!(validate_max_rate("1"), Ok(1));
    }

    #[test]
    fn test_validate_max_rate_typical_value() {
        assert_eq!(validate_max_rate("20"), Ok(20));
    }

    #[test]
    fn test_validate_max_rate_upper_bound() {
        assert_eq!(validate_max_rate("1000"), Ok(1000));
    }

    #[test]
    fn test_validate_max_rate_zero_rejected() {
        assert!(validate_max_rate("0").is_err());
    }

    #[test]
    fn test_validate_max_rate_exceeds_max() {
        assert!(validate_max_rate("1001").is_err());
    }

    fn make_cli(port: u16, max_connections: usize, max_connections_per_ip: usize) -> Cli {
        Cli {
            port: if port > 0 { Some(port) } else { Some(0) },
            ports: Vec::new(),
            bind: Some("127.0.0.1".parse().unwrap()),
            timeout: Some(15),
            max_connections: Some(max_connections),
            max_connections_per_ip: Some(max_connections_per_ip),
            max_rate: Some(20),
            max_rate_per_ip: Some(5),
            allow_cidr: Vec::new(),
            silent_reject: Some(true),
            hold_open: Some(false),
            config: None,
            log_format: None,
            health_check: false,
            health_check_tls: false,
            health_check_domain: None,
            health_check_ca: None,
            admin_port: None,
            admin_token: None,
            admin_token_file: None,
            tls_min_version: None,
            tls_cert: None,
            tls_key: None,
            tls_client_ca: None,
            user: None,
            group: None,
            service: false,
        }
    }

    #[test]
    fn test_cli_validate_valid_config() {
        assert!(make_cli(8080, 100, 10).validate().is_ok());
    }

    #[test]
    fn test_cli_validate_port_zero() {
        assert!(make_cli(0, 100, 10).validate().is_err());
    }

    #[test]
    fn test_cli_validate_per_ip_exceeds_global() {
        assert!(make_cli(8080, 10, 50).validate().is_err());
    }

    #[test]
    fn test_cli_validate_per_ip_equals_global() {
        assert!(make_cli(8080, 100, 100).validate().is_ok());
    }

    #[test]
    fn test_cidr_v4_matching() {
        let cidr = Cidr::parse("10.0.0.0/8").unwrap();
        assert!(cidr.contains(&"10.0.0.1".parse().unwrap()));
        assert!(cidr.contains(&"10.255.255.255".parse().unwrap()));
        assert!(!cidr.contains(&"192.168.1.1".parse().unwrap()));

        let host_cidr = Cidr::parse("192.168.1.50").unwrap();
        assert!(host_cidr.contains(&"192.168.1.50".parse().unwrap()));
        assert!(!host_cidr.contains(&"192.168.1.51".parse().unwrap()));
    }

    #[test]
    fn test_cidr_v6_matching() {
        let cidr = Cidr::parse("2001:db8::/32").unwrap();
        assert!(cidr.contains(&"2001:db8:ffff::1".parse().unwrap()));
        assert!(!cidr.contains(&"2001:db9::1".parse().unwrap()));
    }

    #[test]
    fn test_cidr_whitespace_handling() {
        let cidr = Cidr::parse("  10.0.0.0 / 8  ").unwrap();
        assert!(cidr.contains(&"10.1.2.3".parse().unwrap()));
        let host_cidr = Cidr::parse("  192.168.1.1  ").unwrap();
        assert!(host_cidr.contains(&"192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn test_cidr_invalid_inputs() {
        assert!(Cidr::parse("not-an-ip").is_err());
        assert!(Cidr::parse("10.0.0.1/33").is_err());
        assert!(Cidr::parse("::1/129").is_err());
    }

    #[test]
    fn test_cli_validate_allow_cidr() {
        let mut cli = make_cli(8080, 100, 10);
        cli.allow_cidr = vec!["10.0.0.0/8".to_string(), "192.168.0.0/16".to_string()];
        assert!(cli.validate().is_ok());

        cli.allow_cidr = vec!["invalid-cidr".to_string()];
        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cidr_dual_stack_ipv4_mapped_ipv6() {
        let cidr = Cidr::parse("::ffff:192.168.1.0/120").unwrap();
        assert!(matches!(cidr, Cidr::V4 { .. }));
        assert!(cidr.contains(&"192.168.1.5".parse().unwrap()));
        assert!(cidr.contains(&"::ffff:192.168.1.5".parse().unwrap()));
        assert!(!cidr.contains(&"192.168.2.1".parse().unwrap()));

        let v4_cidr = Cidr::parse("10.0.0.0/8").unwrap();
        assert!(v4_cidr.contains(&"::ffff:10.5.6.7".parse().unwrap()));
    }

    #[test]
    fn test_cli_parse_cidrs_method() {
        let mut cli = make_cli(8080, 100, 10);
        cli.allow_cidr = vec!["172.16.0.0/12".to_string()];
        let parsed = cli.parse_cidrs().unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].contains(&"172.20.1.1".parse().unwrap()));
    }

    #[test]
    fn test_cli_tls_validation() {
        let mut cli = make_cli(8080, 100, 10);
        cli.tls_cert = Some(PathBuf::from("cert.pem"));
        // cert without key must fail
        assert!(cli.validate().is_err());

        cli.tls_key = Some(PathBuf::from("key.pem"));
        // cert and key together must succeed
        assert!(cli.validate().is_ok());

        cli.tls_client_ca = Some(PathBuf::from("ca.pem"));
        // cert, key, and ca together must succeed
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn test_cli_config_file_loading() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("portprobe_test_config.toml");
        let toml_content = r#"
port = 9090
bind = "10.0.0.1"
timeout = 30
max_connections = 500
max_connections_per_ip = 25
max_rate = 50
allow_cidr = ["10.0.0.0/8", "192.168.0.0/16"]
silent_reject = true
hold_open = true
log_format = "json"
admin_port = 9091
admin_token = "TestToken123"
max_rate_per_ip = 10
ports = ["8081", "8082", "8090-8092"]
"#;
        std::fs::write(&config_path, toml_content).unwrap();

        let mut cli = Cli {
            port: None,
            ports: Vec::new(),
            bind: None,
            timeout: None,
            max_connections: None,
            max_connections_per_ip: None,
            max_rate: None,
            max_rate_per_ip: None,
            allow_cidr: Vec::new(),
            silent_reject: None,
            hold_open: None,
            config: Some(config_path.clone()),
            log_format: None,
            health_check: false,
            health_check_tls: false,
            health_check_domain: None,
            health_check_ca: None,
            admin_port: None,
            admin_token: None,
            admin_token_file: None,
            tls_min_version: None,
            tls_cert: None,
            tls_key: None,
            tls_client_ca: None,
            user: None,
            group: None,
            service: false,
        };

        cli.apply_config_file().unwrap();
        cli.resolve_defaults();

        assert_eq!(cli.port, Some(9090));
        assert_eq!(cli.ports.len(), 3);
        let resolved = cli.resolve_all_ports().unwrap();
        assert_eq!(resolved, vec![8081, 8082, 8090, 8091, 8092, 9090]);
        assert_eq!(cli.bind, Some("10.0.0.1".parse::<IpAddr>().unwrap()));
        assert_eq!(cli.timeout, Some(30));
        assert_eq!(cli.max_connections, Some(500));
        assert_eq!(cli.max_connections_per_ip, Some(25));
        assert_eq!(cli.max_rate, Some(50));
        assert_eq!(cli.max_rate_per_ip, Some(10));
        assert_eq!(cli.allow_cidr.len(), 2);
        assert_eq!(cli.silent_reject, Some(true));
        assert_eq!(cli.hold_open, Some(true));
        assert_eq!(cli.log_format, Some(LogFormat::Json));
        assert_eq!(cli.admin_port, Some(9091));
        assert_eq!(cli.admin_token, Some("TestToken123".to_string()));
        assert!(cli.validate().is_ok());

        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_cli_config_precedence_cli_overrides_file() {
        // Explicit CLI parameter --bind 127.0.0.1 must NEVER be overwritten by config file's bind = "0.0.0.0"
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("portprobe_precedence_test.toml");
        let toml_content = r#"
port = 8080
bind = "0.0.0.0"
timeout = 60
max_connections = 1000
silent_reject = false
hold_open = true
"#;
        std::fs::write(&config_path, toml_content).unwrap();

        // Simulate operator explicitly specifying --bind 127.0.0.1 and --silent-reject true on CLI
        let mut cli = Cli {
            port: None,
            ports: Vec::new(),
            bind: Some("127.0.0.1".parse::<IpAddr>().unwrap()),
            timeout: Some(15),
            max_connections: None,
            max_connections_per_ip: None,
            max_rate: None,
            max_rate_per_ip: None,
            allow_cidr: Vec::new(),
            silent_reject: Some(true),
            hold_open: Some(false),
            config: Some(config_path.clone()),
            log_format: None,
            health_check: false,
            health_check_tls: false,
            health_check_domain: None,
            health_check_ca: None,
            admin_port: None,
            admin_token: None,
            admin_token_file: None,
            tls_min_version: None,
            tls_cert: None,
            tls_key: None,
            tls_client_ca: None,
            user: None,
            group: None,
            service: false,
        };

        cli.apply_config_file().unwrap();
        cli.resolve_defaults();

        // Verify that explicit CLI values were preserved and NOT overwritten by file defaults
        assert_eq!(cli.bind, Some("127.0.0.1".parse::<IpAddr>().unwrap()));
        assert_eq!(cli.timeout, Some(15));
        assert_eq!(cli.silent_reject, Some(true));
        assert_eq!(cli.hold_open, Some(false));

        // Unspecified CLI values should be loaded from the file
        assert_eq!(cli.port, Some(8080));
        assert_eq!(cli.max_connections, Some(1000));

        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_cli_resolve_defaults_assigns_secure_defaults() {
        let mut cli = Cli {
            port: Some(8080),
            ports: Vec::new(),
            bind: None,
            timeout: None,
            max_connections: None,
            max_connections_per_ip: None,
            max_rate: None,
            max_rate_per_ip: None,
            allow_cidr: Vec::new(),
            silent_reject: None,
            hold_open: None,
            config: None,
            log_format: None,
            health_check: false,
            health_check_tls: false,
            health_check_domain: None,
            health_check_ca: None,
            admin_port: None,
            admin_token: None,
            admin_token_file: None,
            tls_min_version: None,
            tls_cert: None,
            tls_key: None,
            tls_client_ca: None,
            user: None,
            group: None,
            service: false,
        };

        cli.resolve_defaults();

        assert_eq!(cli.bind, Some("127.0.0.1".parse::<IpAddr>().unwrap()));
        assert_eq!(cli.timeout, Some(15));
        assert_eq!(cli.max_connections, Some(100));
        assert_eq!(cli.max_connections_per_ip, Some(10));
        assert_eq!(cli.max_rate, Some(20));
        assert_eq!(cli.max_rate_per_ip, Some(5));
        // Verify secure-by-default parameters:
        assert_eq!(cli.silent_reject, Some(true)); // Drops reconnaissance banners
        assert_eq!(cli.hold_open, Some(false)); // Releases connection slots immediately
    }

    #[test]
    fn test_resolve_all_ports_exceeds_max_limit_rejected() {
        // Capping total listening ports across ranges and discrete numbers to 50
        let mut cli = make_cli(8080, 100, 10);
        cli.ports = vec!["1000-1060".to_string()]; // 61 ports
        assert!(cli.resolve_all_ports().is_err());
    }

    #[test]
    fn test_cli_validate_tls_min_version() {
        let mut cli = make_cli(8080, 100, 10);
        cli.tls_min_version = Some("1.3".to_string());
        assert!(cli.validate().is_ok());

        cli.tls_min_version = Some("1.1".to_string());
        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cli_validate_admin_token_file() {
        let temp_dir = std::env::temp_dir();
        let token_path = temp_dir.join(format!("test_admin_token_{}.txt", std::process::id()));
        std::fs::write(&token_path, "SecretFromTokenFile123\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&token_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        let mut cli = make_cli(8080, 100, 10);
        cli.admin_token_file = Some(token_path.clone());
        assert!(cli.validate().is_ok());

        cli.resolve_defaults();
        assert_eq!(cli.admin_token, Some("SecretFromTokenFile123".to_string()));

        let _ = std::fs::remove_file(token_path);
    }

    #[cfg(unix)]
    #[test]
    fn test_cli_validate_admin_token_file_insecure_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = std::env::temp_dir();
        let token_path = temp_dir.join(format!(
            "test_admin_token_insecure_{}.txt",
            std::process::id()
        ));
        std::fs::write(&token_path, "SecretFromTokenFile123\n").unwrap();
        std::fs::set_permissions(&token_path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let mut cli = make_cli(8080, 100, 10);
        cli.admin_token_file = Some(token_path.clone());
        let res = cli.validate();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Insecure permissions"));

        let _ = std::fs::remove_file(token_path);
    }

    #[test]
    fn test_validate_max_rate_per_ip_bounds() {
        assert_eq!(validate_max_rate_per_ip("1"), Ok(1));
        assert_eq!(validate_max_rate_per_ip("5"), Ok(5));
        assert_eq!(validate_max_rate_per_ip("1000"), Ok(1000));
        assert!(validate_max_rate_per_ip("0").is_err());
        assert!(validate_max_rate_per_ip("1001").is_err());
        assert!(validate_max_rate_per_ip("not-a-number").is_err());
    }

    #[test]
    fn test_resolve_all_ports_multi_and_range() {
        let mut cli = make_cli(8080, 100, 10);
        cli.ports = vec!["80".to_string(), "443".to_string(), "8085-8088".to_string()];
        let ports = cli.resolve_all_ports().unwrap();
        assert_eq!(ports, vec![80, 443, 8080, 8085, 8086, 8087, 8088]);
    }

    #[test]
    fn test_resolve_all_ports_deduplication() {
        let mut cli = make_cli(8080, 100, 10);
        cli.ports = vec!["8080".to_string(), "8080-8082".to_string()];
        let ports = cli.resolve_all_ports().unwrap();
        assert_eq!(ports, vec![8080, 8081, 8082]);
    }

    #[test]
    fn test_cli_validate_per_ip_rate_exceeds_global() {
        let mut cli = make_cli(8080, 100, 10);
        cli.max_rate = Some(10);
        cli.max_rate_per_ip = Some(20);
        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cli_validate_empty_admin_token() {
        let mut cli = make_cli(8080, 100, 10);
        cli.admin_token = Some("   ".to_string());
        assert!(cli.validate().is_err());

        cli.admin_token = Some("Secret123".to_string());
        assert!(cli.validate().is_ok());
    }
}
