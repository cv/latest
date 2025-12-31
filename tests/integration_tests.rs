use std::process::Command;

fn latest_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_latest"))
}

#[test]
fn test_help_flag() {
    let output = latest_cmd().arg("--help").output().expect("Failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Find the latest version"));
    assert!(stdout.contains("--source"));
    assert!(stdout.contains("--all"));
    assert!(stdout.contains("--json"));
}

#[test]
fn test_nonexistent_package() {
    let output = latest_cmd().arg("nonexistent-package-xyz-12345").output().expect("Failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_nonexistent_package_json() {
    let output = latest_cmd()
        .args(["--json", "nonexistent-package-xyz-12345"])
        .output()
        .expect("Failed to run");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    assert_eq!(json["package"], "nonexistent-package-xyz-12345");
    assert_eq!(json["status"], "not_found");
}

#[test]
fn test_unknown_source() {
    let output =
        latest_cmd().args(["--source", "not_found", "foo"]).output().expect("Failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown source: not_found"));
}

#[test]
fn test_multiple_packages() {
    let output =
        latest_cmd().args(["--source", "cargo", "serde", "clap"]).output().expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("serde:"));
    assert!(stdout.contains("clap:"));
}

#[test]
fn test_multiple_packages_json() {
    let output = latest_cmd()
        .args(["--json", "--source", "cargo", "serde", "clap"])
        .output()
        .expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["package"], "serde");
    assert_eq!(arr[1]["package"], "clap");
}

#[test]
fn test_single_package_json() {
    let output = latest_cmd()
        .args(["--json", "--source", "cargo", "serde"])
        .output()
        .expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    // Single package returns object, not array
    assert!(json.is_object());
    assert_eq!(json["package"], "serde");
    assert_eq!(json["status"], "up_to_date");
    assert!(json["installed"]["version"].is_string());
}

#[test]
fn test_all_flag_json() {
    let output = latest_cmd()
        .args(["--json", "--all", "--source", "cargo", "serde"])
        .output()
        .expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    assert!(json["available"].is_array());
    let available = json["available"].as_array().unwrap();
    assert!(!available.is_empty());
    assert_eq!(available[0]["source"], "cargo");
}

// Test real packages that should exist
#[test]
fn test_cargo_serde() {
    let output = latest_cmd().args(["--source", "cargo", "serde"]).output().expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Should contain a version and checkmark
    assert!(stdout.contains('.'), "Expected version with dots: {}", stdout);
    assert!(stdout.contains('✓'), "Expected checkmark: {}", stdout);
}

#[test]
fn test_npm_express() {
    let output = latest_cmd().args(["--source", "npm", "express"]).output().expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(stdout.contains('.'), "Expected version with dots: {}", stdout);
}

#[test]
fn test_mixed_found_and_not_found() {
    let output = latest_cmd()
        .args(["--source", "cargo", "serde", "nonexistent-xyz-12345"])
        .output()
        .expect("Failed to run");

    // Should fail because one package wasn't found
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // But should still output the found one
    assert!(stdout.contains("serde:"), "stdout: {}", stdout);
    assert!(stderr.contains("not found"), "stderr: {}", stderr);
}

#[test]
fn test_quiet_mode() {
    let output = latest_cmd()
        .args(["--quiet", "--source", "cargo", "serde"])
        .output()
        .expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Quiet mode should just output version, no checkmark
    assert!(!stdout.contains('✓'));
    assert!(stdout.contains('.'));
}

#[test]
fn test_outdated_exit_code() {
    // This test is tricky - we need a package where installed != latest
    // For now, skip this test as it depends on system state
}
