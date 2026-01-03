use super::{Ecosystem, Source};
use std::process::Command;

pub struct CondaSource;

impl Source for CondaSource {
    fn name(&self) -> &'static str {
        "conda"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Python
    }

    fn get_version(&self, package: &str) -> Option<String> {
        // Check if conda is available
        let which = Command::new("which").arg("conda").output().ok()?;
        if !which.status.success() {
            return None;
        }

        let output = Command::new("conda").args(["search", package, "--json"]).output().ok()?;

        if !output.status.success() {
            return None;
        }

        parse_conda_output(&String::from_utf8_lossy(&output.stdout), package)
    }
}

fn parse_conda_output(json: &str, package: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;

    // JSON structure: {"package_name": [{...version info...}, ...]}
    // Versions are sorted, last one is latest
    let versions = parsed.get(package)?.as_array()?;
    let latest = versions.last()?;
    latest.get("version")?.as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_conda_output() {
        let json = r#"{"numpy":[{"version":"1.24.0"},{"version":"1.25.0"},{"version":"1.26.0"}]}"#;
        assert_eq!(parse_conda_output(json, "numpy"), Some("1.26.0".to_string()));
    }

    #[test]
    fn test_parse_conda_output_single_version() {
        let json = r#"{"pandas":[{"version":"2.0.0"}]}"#;
        assert_eq!(parse_conda_output(json, "pandas"), Some("2.0.0".to_string()));
    }

    #[test]
    fn test_parse_conda_output_not_found() {
        let json = r#"{}"#;
        assert_eq!(parse_conda_output(json, "nonexistent"), None);
    }

    #[test]
    fn test_parse_conda_output_empty_versions() {
        let json = r#"{"pkg":[]}"#;
        assert_eq!(parse_conda_output(json, "pkg"), None);
    }

    #[test]
    fn test_parse_conda_output_invalid_json() {
        assert_eq!(parse_conda_output("not json", "pkg"), None);
    }

    #[test]
    fn test_conda_source_properties() {
        let conda = CondaSource;
        assert_eq!(conda.name(), "conda");
        assert_eq!(conda.ecosystem(), Ecosystem::Python);
        assert!(!conda.is_local());
    }
}
