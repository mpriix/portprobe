//! PortProbe — Secure TCP connectivity testing utility.
//!
//! Starts a temporary TCP listener for verifying port reachability
//! without requiring a full application deployment. Designed for
//! SRE, DevOps, and Network Engineering teams.

#![deny(unsafe_code)]

use clap::Parser;
use std::net::{IpAddr, SocketAddr};
use std::process::ExitCode;
use tracing::{error, info, warn};

use portprobe::cli::Cli;
use portprobe::logger;
use portprobe::server::{self, ServerParams};

/// Application entry point.
///
/// Initializes the logger in a **single-threaded context** before
/// starting the Tokio runtime.
fn main() -> ExitCode {
    let mut args = Cli::parse();

    // Handle --service flag explicitly rather than silently ignoring it
    if args.service {
        #[cfg(windows)]
        {
            eprintln!("PortProbe native Windows Service dispatcher requires service manager integration (NSSM recommended for v2.0.0; native SCM dispatch scheduled for v2.2.0)");
            return ExitCode::FAILURE;
        }
        #[cfg(not(windows))]
        {
            eprintln!("--service is only supported on Windows platforms");
            return ExitCode::FAILURE;
        }
    }

    // Merge configuration from file if specified via -c/--config or PORTPROBE_CONFIG
    if let Err(err) = args.apply_config_file() {
        eprintln!("Configuration file error: {}", err);
        return ExitCode::FAILURE;
    }

    // Resolve unconfigured parameters to secure enterprise defaults
    args.resolve_defaults();

    // Initialize logger in single-threaded context with configured format (Text or JSON)
    logger::init_logger(args.log_format.unwrap_or_default());

    // Build the Tokio runtime and run the async entry point.
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("Failed to initialize Tokio async runtime: {}", err);
            return ExitCode::FAILURE;
        }
    };

    runtime.block_on(async_main(args))
}

/// Async entry point — validates configuration, runs healthcheck probe if requested,
/// or starts the TCP server.
async fn async_main(args: Cli) -> ExitCode {
    if let Err(err) = args.validate() {
        eprintln!("Configuration error: {}", err);
        return ExitCode::FAILURE;
    }

    let ports = match args.resolve_all_ports() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Configuration error: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let bind_ip = args
        .bind
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    // If --health-check was requested, execute client probe and exit immediately
    if args.health_check {
        let probe_port = args.port.unwrap_or(ports[0]);
        let bind_address = SocketAddr::new(bind_ip, probe_port);
        let health_tls = if args.health_check_tls {
            Some(server::HealthCheckTlsConfig {
                domain: args
                    .health_check_domain
                    .unwrap_or_else(|| "localhost".to_string()),
                ca_cert: args.health_check_ca,
            })
        } else {
            None
        };
        match server::run_health_check(bind_address, health_tls).await {
            Ok(()) => {
                println!(
                    "PortProbe healthcheck SUCCESS: {} is reachable",
                    bind_address
                );
                return ExitCode::SUCCESS;
            }
            Err(err) => {
                eprintln!("PortProbe healthcheck FAILURE: {}", err);
                return ExitCode::FAILURE;
            }
        }
    }

    // Warn when binding to all interfaces — this exposes the listener to every network.
    if bind_ip.is_unspecified() {
        warn!(
            "Listening on all interfaces. Ensure firewall \
             restrictions are configured."
        );
    }

    // Warn when using any privileged port (< 1024).
    for &port in &ports {
        if port < 1024 {
            warn!(
                "Port {} is in the privileged range (1-1023). \
                 This may require elevated permissions and is \
                 not recommended for diagnostic tools.",
                port
            );
        }
    }

    // Parse CIDR allowlist directly.
    let allow_cidrs = match args.parse_cidrs() {
        Ok(cidrs) => cidrs,
        Err(err) => {
            eprintln!("Configuration error: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if !allow_cidrs.is_empty() {
        info!(
            "CIDR ingress filtering enabled: allowing {} subnet(s)",
            allow_cidrs.len()
        );
    }

    // Warn if --admin-token was passed directly on CLI
    if std::env::args().any(|a| a == "--admin-token" || a.starts_with("--admin-token=")) {
        warn!(
            "Passing --admin-token via CLI arguments exposes credentials in process listings; \
             prefer PORTPROBE_ADMIN_TOKEN environment variable or admin_token_file in configuration"
        );
    }

    // Load TLS acceptor if configured
    let (tls_acceptor, cert_expiry) =
        if let (Some(ref cert), Some(ref key)) = (&args.tls_cert, &args.tls_key) {
            match server::load_tls_acceptor(
                cert,
                key,
                args.tls_client_ca.as_deref(),
                args.tls_min_version.as_deref(),
            ) {
                Ok((acceptor, expiry)) => {
                    info!(
                        "TLS transport encryption enabled with certificate {}",
                        cert.display()
                    );
                    if args.tls_client_ca.is_some() {
                        info!("Mutual TLS (mTLS) client verification enabled");
                    }
                    (Some(acceptor), expiry)
                }
                Err(err) => {
                    error!("Failed to initialize TLS: {}", err);
                    return ExitCode::FAILURE;
                }
            }
        } else {
            (None, None)
        };

    let params = ServerParams {
        bind_ip,
        ports,
        timeout_seconds: args.timeout.unwrap_or(15),
        max_connections: args.max_connections.unwrap_or(100),
        max_connections_per_ip: args.max_connections_per_ip.unwrap_or(10),
        max_rate: args.max_rate.unwrap_or(20),
        max_rate_per_ip: args.max_rate_per_ip.unwrap_or(5),
        allow_cidrs,
        silent_reject: args.silent_reject.unwrap_or(true),
        hold_open: args.hold_open.unwrap_or(false),
        tls_acceptor,
        tls_cert_path: args.tls_cert,
        tls_key_path: args.tls_key,
        tls_client_ca_path: args.tls_client_ca,
        tls_min_version: args.tls_min_version,
        cert_expiry,
        admin_port: args.admin_port,
        admin_token: args.admin_token,
        user: args.user,
        group: args.group,
        config_path: args.config,
    };

    if let Err(err) = server::start_server(params).await {
        error!("PortProbe failed: {}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
