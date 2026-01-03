use super::{Ecosystem, Source};
use std::process::Command;

pub struct NuGetSource;

impl Source for NuGetSource {
    fn name(&self) -> &'static str {
        "nuget"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Dotnet
    }

    fn get_version(&self, package: &str) -> Option<String> {
        // NuGet package IDs are case-insensitive, API requires lowercase
        let url = format!(
            "https://api.nuget.org/v3-flatcontainer/{}/index.json",
            urlencoding::encode(&package.to_lowercase())
        );

        let output = Command::new("curl").args(["-sf", "-m", "10", &url]).output().ok()?;
        if !output.status.success() {
            return None;
        }

        parse_nuget_versions(&String::from_utf8_lossy(&output.stdout))
    }
}

fn parse_nuget_versions(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    let versions = parsed.get("versions")?.as_array()?;

    // Filter out prereleases (contain '-') and get last (latest) stable version
    versions.iter().filter_map(|v| v.as_str()).rfind(|v| !v.contains('-')).map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nuget_versions() {
        let json = r#"{"versions":["1.0.0","1.1.0","2.0.0"]}"#;
        assert_eq!(parse_nuget_versions(json), Some("2.0.0".to_string()));
    }

    #[test]
    fn test_parse_nuget_versions_with_prereleases() {
        let json = r#"{"versions":["1.0.0","2.0.0-beta","2.0.0-rc1","1.5.0"]}"#;
        // Should skip prereleases, return 1.5.0 (last stable)
        assert_eq!(parse_nuget_versions(json), Some("1.5.0".to_string()));
    }

    #[test]
    fn test_parse_nuget_versions_only_prereleases() {
        let json = r#"{"versions":["1.0.0-alpha","1.0.0-beta"]}"#;
        assert_eq!(parse_nuget_versions(json), None);
    }

    #[test]
    fn test_parse_nuget_versions_empty() {
        let json = r#"{"versions":[]}"#;
        assert_eq!(parse_nuget_versions(json), None);
    }

    #[test]
    fn test_parse_nuget_versions_invalid_json() {
        assert_eq!(parse_nuget_versions("not json"), None);
    }

    #[test]
    fn test_nuget_source_properties() {
        let nuget = NuGetSource;
        assert_eq!(nuget.name(), "nuget");
        assert_eq!(nuget.ecosystem(), Ecosystem::Dotnet);
        assert!(!nuget.is_local());
    }
}
