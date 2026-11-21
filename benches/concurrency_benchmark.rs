//! Benchmark: Concurrency scaling performance across 1, 10, 50, 100, 200, 500 tasks.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::{watch, Barrier};

use portprobe::server::{self, ServerParams, RESPONSE};

#[tokio::main]
async fn main() {
    println!("============================================================");
    println!("     PORTPROBE BENCHMARK: CONCURRENCY SCALING EVALUATION");
    println!("============================================================");

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 15,
        max_connections: 2000,
        max_connections_per_ip: 2000,
        max_rate: 10000,
        max_rate_per_ip: 10000,
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

    println!(
        "{:<12} {:<12} {:<16} {:<16}",
        "Concurrency", "Success Rate", "Total Time (ms)", "Throughput (ops/s)"
    );
    println!("{:-<12} {:-<12} {:-<16} {:-<16}", "", "", "", "");

    let levels = [1, 10, 50, 100, 200, 500];
    for &c in &levels {
        let barrier = Arc::new(Barrier::new(c));
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::with_capacity(c);
        let start = Instant::now();

        for _ in 0..c {
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

        let elapsed = start.elapsed();
        let successes = success_count.load(Ordering::Relaxed);
        let throughput = (successes as f64) / elapsed.as_secs_f64();
        let rate_pct = (successes as f64 / c as f64) * 100.0;

        println!(
            "{:<12} {:<11.1}% {:<16.2} {:<16.1}",
            c,
            rate_pct,
            elapsed.as_secs_f64() * 1000.0,
            throughput
        );
    }

    let _ = shutdown_tx.send(true);
    let _ = server_handle.await;
    println!("============================================================");
}
