//! Benchmark: Sustained throughput measurement over time window.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::watch;

use portprobe::server::{self, ServerParams, RESPONSE};

#[tokio::main]
async fn main() {
    println!("============================================================");
    println!("     PORTPROBE BENCHMARK: SUSTAINED THROUGHPUT");
    println!("============================================================");

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    let params = ServerParams {
        bind_ip: "127.0.0.1".parse().unwrap(),
        ports: vec![port],
        timeout_seconds: 5,
        max_connections: 2000,
        max_connections_per_ip: 2000,
        max_rate: 20000,
        max_rate_per_ip: 20000,
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

    let duration = Duration::from_secs(3);
    let running = Arc::new(AtomicBool::new(true));
    let completed_requests = Arc::new(AtomicU64::new(0));

    let workers = 16;
    let mut handles = Vec::new();

    let start_time = Instant::now();

    for _ in 0..workers {
        let running_c = Arc::clone(&running);
        let completed_c = Arc::clone(&completed_requests);

        handles.push(tokio::spawn(async move {
            let mut local_buf = [0u8; 64];
            while running_c.load(Ordering::Relaxed) {
                if let Ok(mut stream) = TcpStream::connect(addr).await {
                    if let Ok(n) = stream.read(&mut local_buf).await {
                        if &local_buf[..n] == RESPONSE {
                            completed_c.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        }));
    }

    tokio::time::sleep(duration).await;
    running.store(false, Ordering::Relaxed);

    for h in handles {
        let _ = h.await;
    }

    let actual_duration = start_time.elapsed();
    let total_completed = completed_requests.load(Ordering::Relaxed);
    let throughput = (total_completed as f64) / actual_duration.as_secs_f64();

    println!(
        "Duration:             {:.2} s",
        actual_duration.as_secs_f64()
    );
    println!("Worker Workers:       {}", workers);
    println!("Completed Probes:     {}", total_completed);
    println!("Throughput:           {:.1} connections/sec", throughput);
    println!(
        "Data Throughput:      {:.2} KB/sec",
        (throughput * (RESPONSE.len() as f64)) / 1024.0
    );
    println!("============================================================");

    let _ = shutdown_tx.send(true);
    let _ = server_handle.await;
}
