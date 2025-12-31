use super::{Ecosystem, Source};
use std::process::Command;

pub struct BrewSource;

impl Source for BrewSource {
    fn name(&self) -> &'static str { "brew" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::System }

    fn get_version(&self, package: &str) -> Option<String> {
        let output = Command::new("brew").args(["info", package, "--json=v2"]).output().ok()?;
        if !output.status.success() { return None; }
        
        let parsed: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).ok()?;
        parsed.get("formulae").and_then(|f| f.get(0)).and_then(|f| f.get("versions")).and_then(|v| v.get("stable")).and_then(|s| s.as_str())
            .or_else(|| parsed.get("casks").and_then(|c| c.get(0)).and_then(|c| c.get("version")).and_then(|v| v.as_str()))
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_brew_json() {
        let brew = BrewSource;
        // Can't easily test get_version without brew, but we can test the JSON paths work
        let formula = r#"{"formulae":[{"versions":{"stable":"1.2.3"}}],"casks":[]}"#;
        let cask = r#"{"formulae":[],"casks":[{"version":"9.9.9"}]}"#;
        
        let parse = |json: &str| -> Option<String> {
            let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
            parsed.get("formulae").and_then(|f| f.get(0)).and_then(|f| f.get("versions")).and_then(|v| v.get("stable")).and_then(|s| s.as_str())
                .or_else(|| parsed.get("casks").and_then(|c| c.get(0)).and_then(|c| c.get("version")).and_then(|v| v.as_str()))
                .map(|s| s.to_string())
        };
        
        assert_eq!(parse(formula), Some("1.2.3".to_string()));
        assert_eq!(parse(cask), Some("9.9.9".to_string()));
        assert_eq!(parse("{}"), None);
        let _ = brew; // silence unused warning
    }
}
