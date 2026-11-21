//! Unit testing suite for PortProbe internal modules and logic.

mod common;

use std::net::IpAddr;
use std::sync::atomic::Ordering;

use portprobe::cli::{Cidr, Cli};
use portprobe::server::{
    constant_time_token_verify, rate_limit_key, Metrics, PerIpRateLimiter, TokenBucket,
};

// ---------------------------------------------------------------------------
// CLI & Parameter Parsing Tests
// ---------------------------------------------------------------------------

#[test]
fn test_cli_default_resolution() {
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

    assert_eq!(cli.bind, Some("127.0.0.1".parse().unwrap()));
    assert_eq!(cli.timeout, Some(15));
    assert_eq!(cli.max_connections, Some(100));
    assert_eq!(cli.max_connections_per_ip, Some(10));
    assert_eq!(cli.max_rate, Some(20));
    assert_eq!(cli.max_rate_per_ip, Some(5));
    assert_eq!(cli.silent_reject, Some(true));
    assert_eq!(cli.hold_open, Some(false));
}

#[test]
fn test_cli_port_expansion_and_dedup() {
    let cli = Cli {
        port: Some(8080),
        ports: vec!["8080".into(), "9000-9003".into(), "8080".into()],
        bind: Some("127.0.0.1".parse().unwrap()),
        timeout: Some(10),
        max_connections: Some(50),
        max_connections_per_ip: Some(5),
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
    };

    let resolved = cli
        .resolve_all_ports()
        .expect("Should resolve ports cleanly");
    assert_eq!(resolved, vec![8080, 9000, 9001, 9002, 9003]);
}

#[test]
fn test_cli_port_range_too_large_rejected() {
    let cli = Cli {
        port: None,
        ports: vec!["1000-1150".into()], // 151 ports > 100 limit
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

    let res = cli.resolve_all_ports();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("exceeds maximum range of 100"));
}

#[test]
fn test_cli_total_listening_ports_cap() {
    // 50 max listening ports limit
    let cli = Cli {
        port: None,
        ports: vec!["1000-1060".into()], // 61 ports > 50 max
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

    let res = cli.resolve_all_ports();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("exceeds maximum limit of 50"));
}

#[test]
fn test_cli_validation_conflicting_limits() {
    let mut cli = Cli {
        port: Some(8080),
        ports: Vec::new(),
        bind: None,
        timeout: Some(15),
        max_connections: Some(10),
        max_connections_per_ip: Some(20), // per-ip > global
        max_rate: Some(20),
        max_rate_per_ip: Some(5),
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

    assert!(cli.validate().is_err());

    cli.max_connections_per_ip = Some(5);
    cli.max_rate = Some(10);
    cli.max_rate_per_ip = Some(15); // per-ip rate > global rate
    assert!(cli.validate().is_err());
}

// ---------------------------------------------------------------------------
// CIDR Ingress Filtering Unit Tests
// ---------------------------------------------------------------------------

#[test]
fn test_cidr_v4_boundaries() {
    let cidr = Cidr::parse("192.168.1.0/24").unwrap();
    assert!(cidr.contains(&"192.168.1.0".parse().unwrap()));
    assert!(cidr.contains(&"192.168.1.1".parse().unwrap()));
    assert!(cidr.contains(&"192.168.1.254".parse().unwrap()));
    assert!(cidr.contains(&"192.168.1.255".parse().unwrap()));
    assert!(!cidr.contains(&"192.168.2.1".parse().unwrap()));
    assert!(!cidr.contains(&"192.168.0.255".parse().unwrap()));
}

#[test]
fn test_cidr_v6_boundaries() {
    let cidr = Cidr::parse("fd00::/16").unwrap();
    assert!(cidr.contains(&"fd00::1".parse().unwrap()));
    assert!(cidr.contains(&"fd00:ffff::1".parse().unwrap()));
    assert!(!cidr.contains(&"fd01::1".parse().unwrap()));
}

#[test]
fn test_cidr_invalid_prefix_lengths() {
    assert!(Cidr::parse("10.0.0.1/33").is_err());
    assert!(Cidr::parse("::1/129").is_err());
    assert!(Cidr::parse("not_an_ip/24").is_err());
}

// ---------------------------------------------------------------------------
// Rate Limiter Unit Tests
// ---------------------------------------------------------------------------

#[test]
fn test_token_bucket_acquire_and_exhaust() {
    let mut bucket = TokenBucket::new(3);
    assert!(bucket.try_acquire());
    assert!(bucket.try_acquire());
    assert!(bucket.try_acquire());
    assert!(!bucket.try_acquire());
}

#[test]
fn test_per_ip_rate_limiter_key_aggregation() {
    let ip1: IpAddr = "2001:db8:1234:5678:0000:0000:0000:0001".parse().unwrap();
    let ip2: IpAddr = "2001:db8:1234:5678:ffff:ffff:ffff:ffff".parse().unwrap();
    let ip3: IpAddr = "2001:db8:1234:5679:0000:0000:0000:0001".parse().unwrap();

    assert_eq!(rate_limit_key(ip1), rate_limit_key(ip2));
    assert_ne!(rate_limit_key(ip1), rate_limit_key(ip3));
}

#[test]
fn test_per_ip_rate_limiter_cleanup_and_bounds() {
    let mut limiter = PerIpRateLimiter::new(10);
    for i in 0..100 {
        let ip: IpAddr = format!("10.0.0.{}", i).parse().unwrap();
        assert!(limiter.try_acquire(&ip));
    }
}

// ---------------------------------------------------------------------------
// Security & Authentication Helper Tests
// ---------------------------------------------------------------------------

#[test]
fn test_constant_time_token_verify_matrix() {
    assert!(constant_time_token_verify(
        "ValidTokenSecret",
        "ValidTokenSecret"
    ));
    assert!(!constant_time_token_verify(
        "ValidTokenSecret",
        "WrongTokenSecret"
    ));
    assert!(!constant_time_token_verify("Short", "LongerTokenSecret"));
    assert!(!constant_time_token_verify("", "ExpectedToken"));
    assert!(constant_time_token_verify("", ""));
}

#[test]
fn test_metrics_prometheus_generation() {
    let metrics = Metrics::new();
    metrics.accepted.fetch_add(100, Ordering::Relaxed);
    metrics.rate_limited.fetch_add(15, Ordering::Relaxed);
    metrics.cidr_rejected.fetch_add(5, Ordering::Relaxed);

    let output = metrics.format_prometheus();
    assert!(output.contains("portprobe_accepted_total 100"));
    assert!(output.contains("portprobe_rate_limited_total 15"));
    assert!(output.contains("portprobe_cidr_rejected_total 5"));
}
