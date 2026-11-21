//! Integration testing suite for PortProbe components, daemon lifecycle, and admin endpoints.

mod common;

use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use portprobe::server::{run_health_check, ServerParams, RESPONSE};

fn make_test_params(
    port: u16,
    admin_port: Option<u16>,
    admin_token: Option<String>,
) -> ServerParams {
    ServerParams {
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
        tls_acceptor: None,
        tls_cert_path: None,
        tls_key_path: None,
        tls_client_ca_path: None,
        tls_min_version: None,
        cert_expiry: None,
        admin_port,
        admin_token,
        user: None,
        group: None,
        config_path: None,
    }
}

#[tokio::test]
async fn test_daemon_startup_and_single_probe() {
    let port = common::get_available_port();
    let params = make_test_params(port, None, None);

    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    // Connect plain client
    let mut stream = TcpStream::connect(addr)
        .await
        .expect("Failed to connect to daemon");
    let mut buf = [0u8; 64];
    let n = stream
        .read(&mut buf)
        .await
        .expect("Failed to read greeting");
    assert_eq!(&buf[..n], RESPONSE);

    // Run official healthcheck probe function
    let health_res = run_health_check(addr, None).await;
    assert!(
        health_res.is_ok(),
        "Healthcheck probe should succeed against running daemon"
    );

    // Initiate graceful shutdown
    let _ = shutdown_tx.send(true);
    let daemon_res = handle.await.expect("Daemon join failed");
    assert!(daemon_res.is_ok(), "Daemon should terminate cleanly");
}

#[tokio::test]
async fn test_client_probe_against_closed_port_fails() {
    let closed_port = common::get_available_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], closed_port));

    let health_res = run_health_check(addr, None).await;
    assert!(
        health_res.is_err(),
        "Healthcheck must fail against closed port"
    );
}

#[tokio::test]
async fn test_admin_endpoint_healthz_and_metrics_auth() {
    let probe_port = common::get_available_port();
    let admin_port = common::get_available_port();
    let token = "EnterpriseAdminToken_2026".to_string();
    let params = make_test_params(probe_port, Some(admin_port), Some(token.clone()));

    let (_addr, shutdown_tx, handle) = common::start_test_daemon(params).await;
    let admin_addr = SocketAddr::from(([127, 0, 0, 1], admin_port));

    // 1. GET /healthz without auth -> 401
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let s = String::from_utf8_lossy(&resp);
        assert!(
            s.starts_with("HTTP/1.1 401 Unauthorized"),
            "Expected 401 without token"
        );
    }

    // 2. GET /healthz with valid Bearer token -> 200 OK
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        let req = format!(
            "GET /healthz HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
            token
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let s = String::from_utf8_lossy(&resp);
        assert!(
            s.starts_with("HTTP/1.1 200 OK"),
            "Expected 200 with valid token"
        );
        assert!(s.contains("OK\n"));
    }

    // 3. HEAD /healthz with valid Bearer token -> 200 OK with empty body
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        let req = format!(
            "HEAD /healthz HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
            token
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let s = String::from_utf8_lossy(&resp);
        assert!(s.starts_with("HTTP/1.1 200 OK"));
        assert!(s.contains("Content-Length: 3"));
    }

    // 4. GET /metrics with valid Bearer token -> 200 OK with Prometheus output
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        let req = format!(
            "GET /metrics HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
            token
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let s = String::from_utf8_lossy(&resp);
        assert!(s.starts_with("HTTP/1.1 200 OK"));
        assert!(s.contains("portprobe_accepted_total"));
    }

    // 5. GET /invalid -> 404 Not Found
    {
        let mut stream = TcpStream::connect(admin_addr).await.unwrap();
        let req = format!(
            "GET /nonexistent HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
            token
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).await.unwrap();
        let s = String::from_utf8_lossy(&resp);
        assert!(s.starts_with("HTTP/1.1 404 Not Found"));
    }

    // Shutdown daemon
    let _ = shutdown_tx.send(true);
    let daemon_res = handle.await.unwrap();
    assert!(daemon_res.is_ok());
}

#[tokio::test]
async fn test_daemon_restart_cycle() {
    let port = common::get_available_port();

    // First cycle
    {
        let params = make_test_params(port, None, None);
        let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], RESPONSE);
        let _ = shutdown_tx.send(true);
        handle.await.unwrap().unwrap();
    }

    // Allow socket cleanup
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Second cycle on same port
    {
        let params = make_test_params(port, None, None);
        let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], RESPONSE);
        let _ = shutdown_tx.send(true);
        handle.await.unwrap().unwrap();
    }
}
