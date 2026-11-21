//! Benchmark: Single connection latency measurement for PortProbe.

use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::watch;

use portprobe::server::{self, ServerParams, RESPONSE};

#[tokio::main]
async fn main() {
    println!("============================================================");
    println!("     PORTPROBE BENCHMARK: SINGLE CONNECTION LATENCY");
    println!("============================================================");

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: 1000,
        max_connections_per_ip: 1000,
        max_rate: 5000,
        max_rate_per_ip: 5000,
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

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let server_handle =
        tokio::spawn(
            async move { server::start_server_with_shutdown(params, Some(shutdown_rx)).await },
        );

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Warmup
    for _ in 0..20 {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let mut buf = [0u8; 64];
            let _ = stream.read(&mut buf).await;
        }
    }

    let iterations = 500;
    let mut latencies_us: Vec<u128> = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let t0 = Instant::now();
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let mut buf = [0u8; 64];
            if let Ok(n) = stream.read(&mut buf).await {
                if &buf[..n] == RESPONSE {
                    let elapsed = t0.elapsed().as_micros();
                    latencies_us.push(elapsed);
                }
            }
        }
    }

    let _ = shutdown_tx.send(true);
    let _ = server_handle.await;

    latencies_us.sort_unstable();
    let count = latencies_us.len();
    assert!(count > 0, "Benchmark must record successful connections");

    let min = latencies_us[0];
    let max = latencies_us[count - 1];
    let sum: u128 = latencies_us.iter().sum();
    let mean = (sum as f64) / (count as f64);
    let p50 = latencies_us[count * 50 / 100];
    let p90 = latencies_us[count * 90 / 100];
    let p95 = latencies_us[count * 95 / 100];
    let p99 = latencies_us[count * 99 / 100];

    println!("Total Samples:        {}", count);
    println!(
        "Min Latency:          {:.2} ms ({} µs)",
        (min as f64) / 1000.0,
        min
    );
    println!(
        "Mean Latency:         {:.2} ms ({:.1} µs)",
        mean / 1000.0,
        mean
    );
    println!(
        "Median (P50) Latency: {:.2} ms ({} µs)",
        (p50 as f64) / 1000.0,
        p50
    );
    println!(
        "P90 Latency:          {:.2} ms ({} µs)",
        (p90 as f64) / 1000.0,
        p90
    );
    println!(
        "P95 Latency:          {:.2} ms ({} µs)",
        (p95 as f64) / 1000.0,
        p95
    );
    println!(
        "P99 Latency:          {:.2} ms ({} µs)",
        (p99 as f64) / 1000.0,
        p99
    );
    println!(
        "Max Latency:          {:.2} ms ({} µs)",
        (max as f64) / 1000.0,
        max
    );
    println!("============================================================");
}
