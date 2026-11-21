//! Shared test environment utilities and mock servers for PortProbe test suite.

#![allow(dead_code, unused_imports)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, watch};

use portprobe::server::{self, ServerParams};

/// Retrieve an available OS-assigned TCP port by temporarily binding to port 0.
pub fn get_available_port() -> u16 {
    let std_listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind temporary port listener");
    let port = std_listener
        .local_addr()
        .expect("Failed to get local address")
        .port();
    drop(std_listener);
    port
}

/// Retrieve path to tests/fixtures directory.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Retrieve path to a certificate in tests/fixtures/certs/.
pub fn cert_path(name: &str) -> PathBuf {
    let path = fixtures_dir().join("certs").join(name);
    #[cfg(unix)]
    if name.ends_with(".key") && path.exists() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    path
}

/// Retrieve path to a config in tests/fixtures/configs/.
pub fn config_path(name: &str) -> PathBuf {
    fixtures_dir().join("configs").join(name)
}

/// Retrieve path to compiled portprobe binary.
pub fn target_bin_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("portprobe");
    #[cfg(windows)]
    let path = path.with_extension("exe");
    path
}

/// Spawn a mock TCP server that sends a specific response and closes.
pub async fn start_mock_server(response: &'static [u8]) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind mock server");
    let addr = listener.local_addr().expect("Failed to get local address");
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    if let Ok((mut stream, _)) = res {
                        let _ = stream.write_all(response).await;
                        let _ = stream.shutdown().await;
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }
    });

    (addr, shutdown_tx)
}

/// Spawn a slow mock TCP server that waits before sending response.
pub async fn start_slow_server(
    delay: Duration,
    response: &'static [u8],
) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind slow server");
    let addr = listener.local_addr().expect("Failed to get local address");
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    if let Ok((mut stream, _)) = res {
                        tokio::time::sleep(delay).await;
                        let _ = stream.write_all(response).await;
                        let _ = stream.shutdown().await;
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }
    });

    (addr, shutdown_tx)
}

/// Spawn an idle mock TCP server that accepts connections and never sends or reads bytes.
pub async fn start_idle_server() -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind idle server");
    let addr = listener.local_addr().expect("Failed to get local address");
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut streams = Vec::new();
        loop {
            tokio::select! {
                res = listener.accept() => {
                    if let Ok((stream, _)) = res {
                        streams.push(stream);
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }
    });

    (addr, shutdown_tx)
}

/// Spawn a mock server that immediately aborts/resets the connection.
pub async fn start_reset_server() -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind reset server");
    let addr = listener.local_addr().expect("Failed to get local address");
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    if let Ok((stream, _)) = res {
                        drop(stream);
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }
    });

    (addr, shutdown_tx)
}

/// Helper to start a real PortProbe daemon in-process with a shutdown watch channel.
pub async fn start_test_daemon(
    mut params: ServerParams,
) -> (
    SocketAddr,
    watch::Sender<bool>,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    let port = if params.ports.is_empty() {
        let p = get_available_port();
        params.ports = vec![p];
        p
    } else {
        params.ports[0]
    };
    let addr = SocketAddr::new(params.bind_ip, port);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let handle =
        tokio::spawn(
            async move { server::start_server_with_shutdown(params, Some(shutdown_rx)).await },
        );

    // Wait briefly for the listener to bind and initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    (addr, shutdown_tx, handle)
}
