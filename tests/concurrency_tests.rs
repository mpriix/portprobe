//! High-concurrency testing suite for PortProbe admission control and performance limits.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::Barrier;

use portprobe::server::{
    ServerParams, CONNECTION_LIMIT_RESPONSE, PER_IP_LIMIT_RESPONSE, RATE_LIMITED_RESPONSE, RESPONSE,
};

#[tokio::test]
async fn test_staged_concurrency_scaling() {
    let port = common::get_available_port();
    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 15,
        max_connections: 1000,
        max_connections_per_ip: 1000,
        max_rate: 1000,
        max_rate_per_ip: 1000,
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

    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    // Concurrency stages: 1, 10, 50, 100, 250
    for concurrency in [1, 10, 50, 100, 250] {
        let barrier = Arc::new(Barrier::new(concurrency));
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::with_capacity(concurrency);
        let start_time = Instant::now();

        for _ in 0..concurrency {
            let barrier_c = Arc::clone(&barrier);
            let success_c = Arc::clone(&success_count);

            tasks.push(tokio::spawn(async move {
                barrier_c.wait().await;
                if let Ok(mut stream) = TcpStream::connect(addr).await {
                    let mut buf = [0u8; 64];
                    if let Ok(n) = stream.read(&mut buf).await {
                        if &buf[..n] == RESPONSE {
                            success_c.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }));
        }

        for t in tasks {
            let _ = t.await;
        }

        let elapsed = start_time.elapsed();
        let successes = success_count.load(Ordering::Relaxed);
        let throughput = (successes as f64) / elapsed.as_secs_f64();

        println!(
            "Stage Concurrency={}: {}/{} succeeded in {:?} ({:.1} conns/sec)",
            concurrency, successes, concurrency, elapsed, throughput
        );

        assert_eq!(
            successes, concurrency,
            "All connections in concurrency level {} must succeed",
            concurrency
        );
    }

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_global_connection_cap_enforcement() {
    let port = common::get_available_port();
    let max_conns = 8;
    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: max_conns,
        max_connections_per_ip: max_conns,
        max_rate: 1000,
        max_rate_per_ip: 1000,
        allow_cidrs: Vec::new(),
        silent_reject: false,
        hold_open: true, // Keep open to occupy slots
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

    // Fill all slots
    let mut holding_streams = Vec::new();
    for _ in 0..max_conns {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], RESPONSE);
        holding_streams.push(stream);
    }

    // Next connection attempt MUST be rejected with CONNECTION_LIMIT_RESPONSE
    let mut overflow_stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = Vec::new();
    let _ = overflow_stream.read_to_end(&mut buf).await;
    assert_eq!(
        buf.as_slice(),
        CONNECTION_LIMIT_RESPONSE,
        "Connection exceeding global limit must receive connection limit banner"
    );

    // Release slots
    drop(holding_streams);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connection should now succeed
    let mut stream_after = TcpStream::connect(addr).await.unwrap();
    let mut buf2 = [0u8; 64];
    let n2 = stream_after.read(&mut buf2).await.unwrap();
    assert_eq!(&buf2[..n2], RESPONSE);

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_per_ip_connection_cap_enforcement() {
    let port = common::get_available_port();
    let per_ip_cap = 4;
    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: 50,
        max_connections_per_ip: per_ip_cap,
        max_rate: 1000,
        max_rate_per_ip: 1000,
        allow_cidrs: Vec::new(),
        silent_reject: false,
        hold_open: true, // Keep open to pin per-IP slots
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

    let mut holding = Vec::new();
    for _ in 0..per_ip_cap {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], RESPONSE);
        holding.push(stream);
    }

    // Attempting (per_ip_cap + 1) connection from same IP (127.0.0.1)
    let mut excess_stream = TcpStream::connect(addr).await.unwrap();
    let mut buf = Vec::new();
    let _ = excess_stream.read_to_end(&mut buf).await;
    assert_eq!(
        buf.as_slice(),
        PER_IP_LIMIT_RESPONSE,
        "Excess connection from same IP must receive PER_IP_LIMIT_RESPONSE"
    );

    drop(holding);
    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}

#[tokio::test]
async fn test_global_rate_limiter_burst_rejection() {
    let port = common::get_available_port();
    let rate = 5;
    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: 100,
        max_connections_per_ip: 100,
        max_rate: rate,
        max_rate_per_ip: 100,
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

    let (addr, shutdown_tx, handle) = common::start_test_daemon(params).await;

    let mut responses = Vec::new();
    // Burst 15 requests immediately
    for _ in 0..15 {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let mut buf = Vec::new();
            let _ = stream.read_to_end(&mut buf).await;
            responses.push(buf);
        }
    }

    let rate_limited_count = responses
        .iter()
        .filter(|r| r.as_slice() == RATE_LIMITED_RESPONSE)
        .count();

    println!(
        "Burst 15 requests on rate=5: {} rate limited",
        rate_limited_count
    );
    assert!(
        rate_limited_count > 0,
        "Burst exceeding rate limit bucket must produce RATE_LIMITED_RESPONSE"
    );

    let _ = shutdown_tx.send(true);
    let _ = handle.await;
}
