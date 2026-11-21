//! CLI binary execution testing suite executing target/debug/portprobe.

mod common;

use portprobe::server::RESPONSE;
use std::process::Command;

fn get_bin() -> std::path::PathBuf {
    let bin = common::target_bin_path();
    if !bin.exists() {
        // Build binary if not already compiled
        let status = Command::new("cargo")
            .args(["build", "--bin", "portprobe"])
            .status()
            .expect("Failed to build portprobe binary");
        assert!(status.success(), "Cargo build failed");
    }
    bin
}

#[test]
fn test_cli_help_flag() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .arg("--help")
        .output()
        .expect("Failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Secure TCP connectivity testing utility"));
    assert!(stdout.contains("--port"));
    assert!(stdout.contains("--timeout"));
    assert!(stdout.contains("--max-connections"));
    assert!(stdout.contains("--health-check"));
}

#[test]
fn test_cli_version_flag() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .arg("--version")
        .output()
        .expect("Failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("portprobe 2.0.0"));
}

#[test]
fn test_cli_missing_port_fails() {
    let bin = get_bin();
    let output = Command::new(&bin).output().expect("Failed to run binary");

    assert!(
        !output.status.success(),
        "Should fail when no port is specified"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Port must be between 1 and 65535")
            || stderr.contains("Configuration error")
    );
}

#[test]
fn test_cli_invalid_port_number_fails() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .args(["--port", "99999"])
        .output()
        .expect("Failed to run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value '99999'"));
}

#[test]
fn test_cli_invalid_timeout_fails() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .args(["--port", "8080", "--timeout", "0"])
        .output()
        .expect("Failed to run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Timeout must be between 1 and 3600"));
}

#[test]
fn test_cli_unknown_flag_fails() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .arg("--unknown-flag-12345")
        .output()
        .expect("Failed to run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_healthcheck_success_exit_code() {
    let bin = get_bin();
    let (addr, _shutdown_tx) = common::start_mock_server(RESPONSE).await;

    let output = tokio::task::spawn_blocking(move || {
        Command::new(&bin)
            .args([
                "--health-check",
                "--bind",
                &addr.ip().to_string(),
                "--port",
                &addr.port().to_string(),
            ])
            .output()
            .expect("Failed to run binary")
    })
    .await
    .expect("Task join failed");

    assert!(
        output.status.success(),
        "Healthcheck against valid server should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PortProbe healthcheck SUCCESS"));
}

#[test]
fn test_cli_healthcheck_failure_exit_code() {
    let bin = get_bin();
    let closed_port = common::get_available_port();

    let output = Command::new(&bin)
        .args([
            "--health-check",
            "--bind",
            "127.0.0.1",
            "--port",
            &closed_port.to_string(),
        ])
        .output()
        .expect("Failed to run binary");

    assert!(
        !output.status.success(),
        "Healthcheck against closed port should exit 1"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("PortProbe healthcheck FAILURE"));
}

#[test]
fn test_cli_service_flag_handled() {
    let bin = get_bin();
    let output = Command::new(&bin)
        .arg("--service")
        .output()
        .expect("Failed to run binary");

    // --service should be caught immediately with non-zero exit code and guidance
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("service") || stderr.contains("Service"));
}
