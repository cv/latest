use super::{Ecosystem, Source};
use std::process::Command;

pub struct GoSource;

impl Source for GoSource {
    fn name(&self) -> &'static str {
        "go"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Go
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let output = Command::new("go")
            .args(["list", "-m", "-versions", package])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        // Format: "module/path v1.0.0 v1.1.0 v1.2.0" - take last
        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .last()
            .filter(|v| v.starts_with('v'))
            .map(|v| v.strip_prefix('v').unwrap_or(v).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_source_properties() {
        assert_eq!(GoSource.name(), "go");
        assert!(!GoSource.is_local());
        assert_eq!(GoSource.ecosystem(), Ecosystem::Go);
    }
}
