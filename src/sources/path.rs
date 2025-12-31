use super::{extract_version, Ecosystem, Source};
use std::process::Command;

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
        Command::new("which").arg(package).output().ok().filter(|o| o.status.success())?;

        for flag in ["--version", "-version", "version", "-V"] {
            if let Ok(output) = Command::new(package).arg(flag).output() {
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
