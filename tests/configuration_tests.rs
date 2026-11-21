//! Configuration testing suite verifying TOML loading, precedence, and error validation.

mod common;

use std::net::IpAddr;
use std::path::PathBuf;

use portprobe::cli::{Cli, ConfigFile};

fn empty_cli(config_path: Option<PathBuf>) -> Cli {
    Cli {
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
        config: config_path,
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
fn test_load_valid_fixture_configuration() {
    let path = common::config_path("valid.toml");
    let cfg = ConfigFile::load_from_path(&path).expect("valid.toml must load successfully");

    assert_eq!(cfg.port, Some(8080));
    assert_eq!(
        cfg.ports,
        Some(vec!["8081".into(), "8082".into(), "8090-8092".into()])
    );
    assert_eq!(cfg.bind, Some("127.0.0.1".parse::<IpAddr>().unwrap()));
    assert_eq!(cfg.timeout, Some(30));
    assert_eq!(cfg.max_connections, Some(500));
    assert_eq!(cfg.max_connections_per_ip, Some(25));
    assert_eq!(cfg.max_rate, Some(50));
    assert_eq!(cfg.max_rate_per_ip, Some(10));
    assert_eq!(cfg.silent_reject, Some(true));
    assert_eq!(cfg.hold_open, Some(false));
    assert_eq!(cfg.admin_port, Some(9091));
    assert_eq!(cfg.admin_token, Some("SecretTestToken123".into()));

    let mut cli = empty_cli(Some(path));
    cli.apply_config_file().unwrap();
    cli.resolve_defaults();
    assert!(cli.validate().is_ok());

    let resolved_ports = cli.resolve_all_ports().unwrap();
    assert_eq!(resolved_ports, vec![8080, 8081, 8082, 8090, 8091, 8092]);
}

#[test]
fn test_load_malformed_toml_rejected() {
    let path = common::config_path("malformed.toml");
    let res = ConfigFile::load_from_path(&path);
    assert!(res.is_err(), "Malformed TOML must return parse error");
    assert!(res.unwrap_err().contains("Failed to parse TOML"));
}

#[test]
fn test_load_nonexistent_config_rejected() {
    let path = PathBuf::from("nonexistent_config_file_2026.toml");
    let res = ConfigFile::load_from_path(&path);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Configuration file not found"));
}

#[test]
fn test_fixture_invalid_port_range_rejected() {
    let path = common::config_path("invalid_ranges.toml");
    let mut cli = empty_cli(Some(path));
    cli.apply_config_file().unwrap();
    cli.resolve_defaults();
    let res = cli.resolve_all_ports();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("exceeds maximum range of 100"));
}

#[test]
fn test_fixture_invalid_cidr_rejected() {
    let path = common::config_path("invalid_cidr.toml");
    let mut cli = empty_cli(Some(path));
    cli.apply_config_file().unwrap();
    cli.resolve_defaults();
    assert!(cli.validate().is_err());
}

#[test]
fn test_fixture_conflicting_limits_rejected() {
    let path = common::config_path("conflicting_limits.toml");
    let mut cli = empty_cli(Some(path));
    cli.apply_config_file().unwrap();
    cli.resolve_defaults();
    let res = cli.validate();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("cannot exceed global limit"));
}

#[test]
fn test_fixture_conflicting_rate_rejected() {
    let path = common::config_path("conflicting_rate.toml");
    let mut cli = empty_cli(Some(path));
    cli.apply_config_file().unwrap();
    cli.resolve_defaults();
    let res = cli.validate();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("cannot exceed global rate limit"));
}

#[test]
fn test_fixture_tls_missing_key_rejected() {
    let path = common::config_path("tls_missing_key.toml");
    let mut cli = empty_cli(Some(path));
    cli.apply_config_file().unwrap();
    cli.resolve_defaults();
    let res = cli.validate();
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("Both --tls-cert and --tls-key must be specified together"));
}

#[test]
fn test_cli_explicit_precedence_over_config_file() {
    let path = common::config_path("valid.toml");
    let mut cli = empty_cli(Some(path));
    // Explicit CLI override
    cli.port = Some(9999);
    cli.bind = Some("10.10.10.10".parse().unwrap());
    cli.timeout = Some(120);

    cli.apply_config_file().unwrap();
    cli.resolve_defaults();

    // Verify explicit CLI parameters were preserved
    assert_eq!(cli.port, Some(9999));
    assert_eq!(cli.bind, Some("10.10.10.10".parse().unwrap()));
    assert_eq!(cli.timeout, Some(120));

    // Unspecified fields should come from valid.toml
    assert_eq!(cli.max_connections, Some(500));
    assert_eq!(cli.admin_port, Some(9091));
}
