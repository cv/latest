use super::{Ecosystem, Source};
use std::process::Command;

pub struct BrewSource;

impl Source for BrewSource {
    fn name(&self) -> &'static str {
        "brew"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::System
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let output = Command::new("brew")
            .args(["info", package, "--json=v2"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json = String::from_utf8_lossy(&output.stdout);
        parse_brew_json(&json)
    }
}

fn parse_brew_json(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    
    // Try formulae first, then casks
    parsed.get("formulae")
        .and_then(|f| f.get(0))
        .and_then(|f| f.get("versions"))
        .and_then(|v| v.get("stable"))
        .and_then(|s| s.as_str())
        .or_else(|| {
            parsed.get("casks")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("version"))
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brew_source_properties() {
        assert_eq!(BrewSource.name(), "brew");
        assert!(!BrewSource.is_local());
        assert_eq!(BrewSource.ecosystem(), Ecosystem::System);
    }

    #[test]
    fn test_parse_brew_json_formula() {
        let json = r#"{"formulae":[{"versions":{"stable":"1.2.3"}}],"casks":[]}"#;
        assert_eq!(parse_brew_json(json), Some("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_brew_json_cask() {
        let json = r#"{"formulae":[],"casks":[{"version":"9.9.9"}]}"#;
        assert_eq!(parse_brew_json(json), Some("9.9.9".to_string()));
    }

    #[test]
    fn test_parse_brew_json_empty() {
        assert_eq!(parse_brew_json("{}"), None);
        assert_eq!(parse_brew_json(r#"{"formulae":[],"casks":[]}"#), None);
    }
}
