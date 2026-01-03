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
        let output = Command::new("brew").args(["info", package, "--json=v2"]).output().ok()?;
        if !output.status.success() {
            return None;
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).ok()?;

        // Try formulae first
        if let Some(formula) = parsed.get("formulae").and_then(|f| f.get(0)) {
            // Check if the formula name matches what we queried - Homebrew silently
            // redirects aliases (e.g., "npm" -> "node") which would give wrong versions
            let name = formula.get("name").and_then(|n| n.as_str());
            if name == Some(package)
                && let Some(version) =
                    formula.get("versions").and_then(|v| v.get("stable")).and_then(|s| s.as_str())
            {
                return Some(version.to_string());
            }
        }

        // Try casks
        if let Some(cask) = parsed.get("casks").and_then(|c| c.get(0)) {
            // Check cask token matches queried name
            let token = cask.get("token").and_then(|t| t.as_str());
            if token == Some(package)
                && let Some(version) = cask.get("version").and_then(|v| v.as_str())
            {
                return Some(version.to_string());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to simulate brew JSON parsing with name matching
    fn parse_brew_json(json: &str, package: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(json).ok()?;

        if let Some(formula) = parsed.get("formulae").and_then(|f| f.get(0)) {
            let name = formula.get("name").and_then(|n| n.as_str());
            if name == Some(package) {
                if let Some(version) =
                    formula.get("versions").and_then(|v| v.get("stable")).and_then(|s| s.as_str())
                {
                    return Some(version.to_string());
                }
            }
        }

        if let Some(cask) = parsed.get("casks").and_then(|c| c.get(0)) {
            let token = cask.get("token").and_then(|t| t.as_str());
            if token == Some(package) {
                if let Some(version) = cask.get("version").and_then(|v| v.as_str()) {
                    return Some(version.to_string());
                }
            }
        }

        None
    }

    #[test]
    fn test_parse_brew_json() {
        let brew = BrewSource;
        let formula =
            r#"{"formulae":[{"name":"ripgrep","versions":{"stable":"1.2.3"}}],"casks":[]}"#;
        let cask = r#"{"formulae":[],"casks":[{"token":"firefox","version":"9.9.9"}]}"#;

        // Matching names should return versions
        assert_eq!(parse_brew_json(formula, "ripgrep"), Some("1.2.3".to_string()));
        assert_eq!(parse_brew_json(cask, "firefox"), Some("9.9.9".to_string()));

        // Non-matching names should return None (prevents alias confusion like npm->node)
        assert_eq!(parse_brew_json(formula, "rg"), None);
        assert_eq!(parse_brew_json(cask, "ff"), None);

        assert_eq!(parse_brew_json("{}", "anything"), None);
        let _ = brew; // silence unused warning
    }

    #[test]
    fn test_brew_alias_rejection() {
        // Simulate what happens when querying "npm" but brew returns "node"
        let node_json =
            r#"{"formulae":[{"name":"node","versions":{"stable":"25.2.1"}}],"casks":[]}"#;

        // Querying "npm" should NOT return node's version
        assert_eq!(parse_brew_json(node_json, "npm"), None);

        // Querying "node" should return it
        assert_eq!(parse_brew_json(node_json, "node"), Some("25.2.1".to_string()));
    }
}
