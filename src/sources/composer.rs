use super::{Ecosystem, Source};
use std::process::Command;

pub struct ComposerSource;

impl Source for ComposerSource {
    fn name(&self) -> &'static str {
        "composer"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Php
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let url = format!("https://repo.packagist.org/p2/{}.json", urlencoding::encode(package));
        let output = Command::new("curl").args(["-sf", "-m", "10", &url]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        parse_composer_response(&String::from_utf8_lossy(&output.stdout), package)
    }
}

fn parse_composer_response(json: &str, package: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    let packages = parsed.get("packages")?;
    let versions = packages.get(package)?;
    let latest = versions.get(0)?;
    let version = latest.get("version")?.as_str()?;
    // Strip leading 'v' if present
    Some(version.strip_prefix('v').unwrap_or(version).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_composer_response() {
        let json = r#"{"packages":{"monolog/monolog":[{"version":"3.5.0"},{"version":"3.4.0"}]}}"#;
        assert_eq!(parse_composer_response(json, "monolog/monolog"), Some("3.5.0".to_string()));
    }

    #[test]
    fn test_parse_composer_response_with_v_prefix() {
        let json = r#"{"packages":{"test/pkg":[{"version":"v2.0.0"}]}}"#;
        assert_eq!(parse_composer_response(json, "test/pkg"), Some("2.0.0".to_string()));
    }

    #[test]
    fn test_parse_composer_response_not_found() {
        let json = r#"{"packages":{}}"#;
        assert_eq!(parse_composer_response(json, "not/found"), None);
    }

    #[test]
    fn test_composer_source_properties() {
        let composer = ComposerSource;
        assert_eq!(composer.name(), "composer");
        assert_eq!(composer.ecosystem(), Ecosystem::Php);
        assert!(!composer.is_local());
    }
}
