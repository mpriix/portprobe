//! TCP networking test suite covering reachability, edge cases, TLS, and network failures.

mod common;

use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use portprobe::server::{
    load_tls_acceptor, run_health_check, HealthCheckTlsConfig, ServerParams, RESPONSE,
};

#[tokio::test]
async fn test_successful_tcp_connection() {
    let (addr, _shutdown_tx) = common::start_mock_server(RESPONSE).await;
    let mut stream = TcpStream::connect(addr).await.expect("Failed to connect");
    let mut buf = [0u8; 64];
    let n = stream.read(&mut buf).await.expect("Failed to read");
    assert_eq!(&buf[..n], RESPONSE);
}

#[tokio::test]
async fn test_connection_refused() {
    let closed_port = common::get_available_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], closed_port));
    let res = TcpStream::connect(addr).await;
    assert!(res.is_err(), "Expected connection refused on closed port");
}

#[tokio::test]
async fn test_connection_timeout_handling() {
    // Idle server accepts connection but never responds
    let (addr, _shutdown_tx) = common::start_idle_server().await;
    let mut stream = TcpStream::connect(addr).await.expect("Connect succeeded");

    let mut buf = [0u8; 64];
    // Attempt read with short 100ms timeout
    let read_res = tokio::time::timeout(Duration::from_millis(100), stream.read(&mut buf)).await;
    assert!(read_res.is_err(), "Expected read timeout on idle endpoint");
}

#[tokio::test]
async fn test_connection_reset_by_peer() {
    // Reset server accepts connection and immediately drops/aborts socket
    let (addr, _shutdown_tx) = common::start_reset_server().await;
    let mut stream = TcpStream::connect(addr).await.expect("Connect succeeded");

    let mut buf = [0u8; 64];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    assert_eq!(
        n, 0,
        "Expected 0 bytes (EOF/reset) when server drops connection"
    );
}

#[tokio::test]
async fn test_slow_tcp_server_latency() {
    let (addr, _shutdown_tx) =
        common::start_slow_server(Duration::from_millis(200), RESPONSE).await;

    // Fast timeout (50ms) should fail
    {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 64];
        let res = tokio::time::timeout(Duration::from_millis(50), stream.read(&mut buf)).await;
        assert!(res.is_err(), "Should time out before slow server responds");
    }

    // Sufficient timeout (500ms) should succeed
    {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 64];
        let res = tokio::time::timeout(Duration::from_millis(500), stream.read(&mut buf)).await;
        assert!(res.is_ok(), "Should read successfully within 500ms");
        let n = res.unwrap().unwrap();
        assert_eq!(&buf[..n], RESPONSE);
    }
}

#[tokio::test]
async fn test_multi_port_reachability() {
    let p1 = common::get_available_port();
    let p2 = common::get_available_port();
    let p3 = common::get_available_port();

    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![p1, p2, p3],
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
        admin_port: None,
        admin_token: None,
        user: None,
        group: None,
        config_path: None,
    };

    let (_addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    // Verify all 3 ports accept connections and return success banner
    for port in [p1, p2, p3] {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let res = run_health_check(addr, None).await;
        assert!(res.is_ok(), "Port {} should be reachable and healthy", port);
    }

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_mixture_reachable_and_unreachable_endpoints() {
    let (alive_addr, _shutdown_tx) = common::start_mock_server(RESPONSE).await;
    let dead_port = common::get_available_port();
    let dead_addr = SocketAddr::from(([127, 0, 0, 1], dead_port));

    let targets = vec![alive_addr, dead_addr];
    let mut results = Vec::new();

    for target in targets {
        let is_healthy = run_health_check(target, None).await.is_ok();
        results.push(is_healthy);
    }

    assert_eq!(results, vec![true, false]);
}

#[tokio::test]
async fn test_tls_transport_reachability() {
    let cert_file = common::cert_path("server.crt");
    let key_file = common::cert_path("server.key");
    let ca_file = common::cert_path("ca.crt");

    let (tls_acceptor, expiry) = load_tls_acceptor(&cert_file, &key_file, None, Some("1.3"))
        .expect("Failed to initialize TLS acceptor");
    assert!(expiry.is_some());

    let port = common::get_available_port();
    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: 50,
        max_connections_per_ip: 20,
        max_rate: 100,
        max_rate_per_ip: 50,
        allow_cidrs: Vec::new(),
        silent_reject: false,
        hold_open: false,
        tls_acceptor: Some(tls_acceptor),
        tls_cert_path: Some(cert_file.clone()),
        tls_key_path: Some(key_file.clone()),
        tls_client_ca_path: None,
        tls_min_version: Some("1.3".into()),
        cert_expiry: expiry,
        admin_port: None,
        admin_token: None,
        user: None,
        group: None,
        config_path: None,
    };

    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    // 1. Plain TCP client connecting to TLS server should fail healthcheck (no banner or handshake error)
    let plain_check = run_health_check(addr, None).await;
    assert!(
        plain_check.is_err(),
        "Plain TCP probe must fail against TLS-enabled server"
    );

    // 2. TLS healthcheck probe with trusted CA should succeed
    let tls_config = HealthCheckTlsConfig {
        domain: "localhost".to_string(),
        ca_cert: Some(ca_file),
    };
    let tls_check = run_health_check(addr, Some(tls_config)).await;
    assert!(
        tls_check.is_ok(),
        "TLS probe with trusted CA should succeed: {:?}",
        tls_check.err()
    );

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_mtls_client_verification() {
    let cert_file = common::cert_path("server.crt");
    let key_file = common::cert_path("server.key");
    let ca_file = common::cert_path("ca.crt");

    // Load server acceptor with client CA required (mTLS)
    let (tls_acceptor, expiry) =
        load_tls_acceptor(&cert_file, &key_file, Some(&ca_file), Some("1.3"))
            .expect("Failed to initialize mTLS acceptor");

    let port = common::get_available_port();
    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: 50,
        max_connections_per_ip: 20,
        max_rate: 100,
        max_rate_per_ip: 50,
        allow_cidrs: Vec::new(),
        silent_reject: false,
        hold_open: false,
        tls_acceptor: Some(tls_acceptor),
        tls_cert_path: Some(cert_file),
        tls_key_path: Some(key_file),
        tls_client_ca_path: Some(ca_file.clone()),
        tls_min_version: Some("1.3".into()),
        cert_expiry: expiry,
        admin_port: None,
        admin_token: None,
        user: None,
        group: None,
        config_path: None,
    };

    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    // Client without client certificate should fail mTLS handshake
    let probe_without_cert = run_health_check(
        addr,
        Some(HealthCheckTlsConfig {
            domain: "localhost".into(),
            ca_cert: Some(ca_file),
        }),
    )
    .await;
    assert!(
        probe_without_cert.is_err(),
        "Client without client certificate must be rejected when mTLS is enforced"
    );

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}
