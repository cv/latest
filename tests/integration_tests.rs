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
    // Should contain source and version
    assert!(stdout.contains("cargo:"), "Expected source prefix: {}", stdout);
    assert!(stdout.contains('.'), "Expected version with dots: {}", stdout);
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

// Test prefix syntax (npm:express)
#[test]
fn test_prefix_syntax() {
    let output = latest_cmd().arg("npm:express").output().expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Should show npm source
    assert!(stdout.contains("npm:"), "Expected npm source: {}", stdout);
    assert!(stdout.contains('.'), "Expected version with dots: {}", stdout);
}

#[test]
fn test_prefix_syntax_cargo() {
    let output = latest_cmd().arg("cargo:serde").output().expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(stdout.contains("cargo:"), "Expected cargo source: {}", stdout);
}

#[test]
fn test_source_in_output() {
    let output = latest_cmd().arg("npm:express").output().expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Output should always include source prefix
    assert!(stdout.starts_with("npm:"), "Output should start with source: {}", stdout);
}

#[test]
fn test_json_includes_source() {
    let output =
        latest_cmd().args(["--json", "npm:express"]).output().expect("Failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    // JSON should include source field
    assert_eq!(json["installed"]["source"], "npm");
}

// ─────────────────────────────────────────────────────────────────────────────
// Offline mode tests (TDD: tests written first for issue latest-8y4)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_offline_flag_in_help() {
    let output = latest_cmd().arg("--help").output().expect("Failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--offline"), "Help should mention --offline flag");
}

#[test]
fn test_offline_does_not_query_network_sources() {
    // In offline mode, querying a package that only exists in network registries
    // should return "not found" since we only check local sources
    let output = latest_cmd()
        .args(["--offline", "express"])  // express is only in npm (network)
        .output()
        .expect("Failed to run");

    // Should not find it since npm is a network source
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"), "Should not find network-only package in offline mode");
}

#[test]
fn test_offline_json_output() {
    let output = latest_cmd()
        .args(["--offline", "--json", "nonexistent-pkg-xyz"])
        .output()
        .expect("Failed to run");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    assert_eq!(json["status"], "not_found");
}

#[test]
fn test_offline_with_source_override() {
    // Even with --offline, if --source specifies a network source,
    // it should be filtered out (offline takes precedence)
    let output = latest_cmd()
        .args(["--offline", "--source", "npm", "express"])
        .output()
        .expect("Failed to run");

    // npm is a network source, so should be filtered out in offline mode
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}
