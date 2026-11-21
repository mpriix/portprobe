//! Security testing suite validating access controls, DoS resistance, and timing invariant.

mod common;

use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use portprobe::cli::Cidr;
use portprobe::server::{constant_time_token_verify, ServerParams, FORBIDDEN_RESPONSE};

#[tokio::test]
async fn test_cidr_allowlist_and_silent_reject_behavior() {
    let port = common::get_available_port();

    // 1. Silent reject mode: unauthorized client receives 0 bytes
    {
        let params = ServerParams {
            bind_ip: "127.0.0.1".parse().unwrap(),
            ports: vec![port],
            timeout_seconds: 5,
            max_connections: 50,
            max_connections_per_ip: 20,
            max_rate: 100,
            max_rate_per_ip: 50,
            allow_cidrs: vec![Cidr::parse("10.0.0.0/8").unwrap()], // 127.0.0.1 is not in 10.0.0.0/8
            silent_reject: true,
            hold_open: false,
            tls_acceptor: None,
            tls_cert_path: None,
            tls_key_path: None,
            tls_client_ca_path: None,
            tls_min_version: None,
            cert_expiry: None,
            admin_port: None,
            admin_token: None,
            user: None,
            group: None,
            config_path: None,
        };

        let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf).await;
        assert!(
            buf.is_empty(),
            "Silent reject must return 0 bytes to prevent reconnaissance"
        );

        let _ = shutdown_tx.send(true);
        let _ = handle.await;
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    // 2. Non-silent reject mode: unauthorized client receives diagnostic banner
    {
        let params = ServerParams {
            bind_ip: "127.0.0.1".parse().unwrap(),
            ports: vec![port],
            timeout_seconds: 5,
            max_connections: 50,
            max_connections_per_ip: 20,
            max_rate: 100,
            max_rate_per_ip: 50,
            allow_cidrs: vec![Cidr::parse("10.0.0.0/8").unwrap()],
            silent_reject: false,
            hold_open: false,
            tls_acceptor: None,
            tls_cert_path: None,
            tls_key_path: None,
            tls_client_ca_path: None,
            tls_min_version: None,
            cert_expiry: None,
            admin_port: None,
            admin_token: None,
            user: None,
            group: None,
            config_path: None,
        };

        let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf).await;
        assert_eq!(
            buf.as_slice(),
            FORBIDDEN_RESPONSE,
            "Non-silent mode must transmit FORBIDDEN_RESPONSE"
        );

        let _ = shutdown_tx.send(true);
        let _ = handle.await;
    }
}

#[tokio::test]
async fn test_admin_endpoint_slowloris_defense() {
    let probe_port = common::get_available_port();
    let admin_port = common::get_available_port();
    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![probe_port],
        timeout_seconds: 5,
        max_connections: 50,
        max_connections_per_ip: 20,
        max_rate: 100,
        max_rate_per_ip: 50,
        allow_cidrs: Vec::new(),
        silent_reject: false,
        hold_open: false,
        tls_acceptor: None,
        tls_cert_path: None,
        tls_key_path: None,
        tls_client_ca_path: None,
        tls_min_version: None,
        cert_expiry: None,
        admin_port: Some(admin_port),
        admin_token: None,
        user: None,
        group: None,
        config_path: None,
    };

    let (_addr, shutdown_tx, handle) = common::start_test_daemon(params).await;
    let admin_addr = SocketAddr::from(([127, 0, 0, 1], admin_port));

    let mut stream = TcpStream::connect(admin_addr).await.unwrap();
    // Send 4100 bytes of headers without \r\n\r\n
    let big_header = vec![b'X'; 4100];
    let _ = stream.write_all(&big_header).await;
    let _ = stream.shutdown().await;

    let mut resp = Vec::new();
    let _ = stream.read_to_end(&mut resp).await;
    let resp_str = String::from_utf8_lossy(&resp);
    assert!(
        resp_str.starts_with("HTTP/1.1 431"),
        "Admin server must return 431 Request Header Fields Too Large on header flooding"
    );

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_constant_time_token_verify_properties() {
    let token = "EnterpriseProductionSecret2026";
    let candidate_wrong = "EnterpriseProductionSecret2027";
    let candidate_early_diff = "XnterpriseProductionSecret2026";

    assert!(constant_time_token_verify(token, token));
    assert!(!constant_time_token_verify(candidate_wrong, token));
    assert!(!constant_time_token_verify(candidate_early_diff, token));
    assert!(!constant_time_token_verify("short", token));
}
