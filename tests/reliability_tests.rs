//! Reliability, stability, and resource cleanup testing suite.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use portprobe::server::{ServerParams, RESPONSE};

fn make_reliable_params(port: u16, hold_open: bool) -> ServerParams {
    ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: 100,
        max_connections_per_ip: 50,
        max_rate: 500,
        max_rate_per_ip: 500,
        allow_cidrs: Vec::new(),
        silent_reject: false,
        hold_open,
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
    }
}

#[tokio::test]
async fn test_sustained_repeated_connections() {
    let port = common::get_available_port();
    let params = make_reliable_params(port, false);
    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    let iterations = 300;
    let mut success = 0;
    let start = Instant::now();

    for _ in 0..iterations {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let mut buf = [0u8; 64];
            if let Ok(n) = stream.read(&mut buf).await {
                if &buf[..n] == RESPONSE {
                    success += 1;
                }
            }
        }
    }

    let elapsed = start.elapsed();
    println!(
        "Completed {} sequential iterations in {:?} ({:.1} conns/sec)",
        iterations,
        elapsed,
        (iterations as f64) / elapsed.as_secs_f64()
    );
    assert_eq!(
        success, iterations,
        "All 300 sequential connections must succeed"
    );

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_rapid_client_disconnect_drain() {
    let port = common::get_available_port();
    let params = make_reliable_params(port, true); // hold_open enabled
    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    // Connect and immediately drop 50 client connections
    for _ in 0..50 {
        let stream = TcpStream::connect(addr).await.unwrap();
        drop(stream);
    }

    // Give server a moment to detect peer EOF and release slots
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify server is fully responsive and slots are free
    let mut probe_stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = [0u8; 64];
    let n = probe_stream.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], RESPONSE);

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_graceful_shutdown_under_active_load() {
    let port = common::get_available_port();
    let params = make_reliable_params(port, true); // hold_open enabled
    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    let active_connections = 10;
    let barrier = Arc::new(tokio::sync::Barrier::new(active_connections + 1));
    let mut client_tasks = Vec::new();

    for _ in 0..active_connections {
        let barrier_c = Arc::clone(&barrier);
        client_tasks.push(tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], RESPONSE);

            barrier_c.wait().await;
            // Hold connection open until server closes it on shutdown
            let mut drain = [0u8; 16];
            let _ = stream.read(&mut drain).await;
        }));
    }

    // Wait until all 10 clients have connected and read the greeting
    barrier.wait().await;

    // Fire graceful shutdown while clients are still connected
    let shutdown_start = Instant::now();
    let _ = shutdown_tx.send(true);

    // Daemon should terminate and join cleanly
    let daemon_res = handle.await.expect("Daemon join failed");
    assert!(
        daemon_res.is_ok(),
        "Daemon should shut down gracefully without panic"
    );
    let shutdown_duration = shutdown_start.elapsed();

    println!(
        "Graceful shutdown under active load took {:?}",
        shutdown_duration
    );
    assert!(
        shutdown_duration < Duration::from_secs(3),
        "Graceful shutdown should complete quickly"
    );

    for t in client_tasks {
        let _ = t.await;
    }
}

#[tokio::test]
async fn test_hold_open_excessive_payload_drain_cap() {
    let port = common::get_available_port();
    let params = make_reliable_params(port, true); // hold_open enabled
    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = [0u8; 64];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], RESPONSE);

    // Send payload exceeding 16KB cap
    let big_payload = vec![b'P'; 20 * 1024];
    let _ = stream.write_all(&big_payload).await;

    let start = Instant::now();
    // Server should terminate connection immediately due to MAX_HOLD_OPEN_DRAIN_BYTES
    let mut eof_buf = [0u8; 16];
    let res = stream.read(&mut eof_buf).await;
    assert_eq!(
        res.unwrap_or(0),
        0,
        "Server must close stream when drain cap exceeded"
    );
    assert!(start.elapsed() < Duration::from_secs(2));

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}
