use super::{Ecosystem, Source, extract_version};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// Timeout for external command execution (5 seconds)
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

/// Poll interval when waiting for command completion
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Run a command with a timeout. Returns None if the command times out or fails.
fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Option<Output> {
    let mut child = cmd.spawn().ok()?;
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process exited, collect output
                // Note: stdout/stderr were already captured, we just need to read them
                let output = child.wait_with_output().ok()?;
                return Some(output);
            }
            Ok(None) => {
                // Still running, check timeout
                if start.elapsed() > timeout {
                    // Kill the process and return None
                    let _ = child.kill();
                    let _ = child.wait(); // Reap the zombie
                    return None;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(_) => return None,
        }
    }
}

pub struct PathSource;

impl Source for PathSource {
    fn name(&self) -> &'static str {
        "path"
    }
    fn is_local(&self) -> bool {
        true
    }
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::System
    }

    fn get_version(&self, package: &str) -> Option<String> {
        // Check if command exists (which is fast, no timeout needed)
        Command::new("which").arg(package).output().ok().filter(|o| o.status.success())?;

        for flag in ["--version", "-version", "version", "-V"] {
            let mut cmd = Command::new(package);
            cmd.arg(flag);
            if let Some(output) = run_with_timeout(cmd, COMMAND_TIMEOUT) {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if let Some(v) = extract_version(&stdout).or_else(|| extract_version(&stderr)) {
                    return Some(v);
                }
            }
        }
        Some("installed".to_string()) // Command exists but version unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_with_timeout_fast_command() {
        // `echo` should complete well within timeout
        let output = run_with_timeout(Command::new("echo"), Duration::from_secs(5));
        assert!(output.is_some());
    }

    #[test]
    fn test_run_with_timeout_times_out() {
        // `sleep 10` with a 100ms timeout should time out
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let start = Instant::now();
        let output = run_with_timeout(cmd, Duration::from_millis(100));
        let elapsed = start.elapsed();

        assert!(output.is_none(), "Should have timed out");
        assert!(
            elapsed < Duration::from_secs(1),
            "Should have timed out quickly, took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_path_source_properties() {
        let source = PathSource;
        assert_eq!(source.name(), "path");
        assert!(source.is_local());
        assert_eq!(source.ecosystem(), Ecosystem::System);
    }
}
