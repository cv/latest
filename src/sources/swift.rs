use super::{Ecosystem, Source};
use std::process::Command;

pub struct SwiftSource;

impl Source for SwiftSource {
    fn name(&self) -> &'static str {
        "swift"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Swift
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let (owner, repo) = parse_github_repo(package)?;
        let url = format!("https://api.github.com/repos/{owner}/{repo}/tags");

        let output = Command::new("curl")
            .args(["-sf", "-m", "10", &url])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        parse_github_tags(&String::from_utf8_lossy(&output.stdout))
    }
}

fn parse_github_repo(package: &str) -> Option<(String, String)> {
    let cleaned = package
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("github.com/")
        .trim_end_matches(".git")
        .trim_end_matches('/');

    let mut parts = cleaned.split('/');
    let owner = parts.next().filter(|s| !s.is_empty())?;
    let repo = parts.next().filter(|s| !s.is_empty())?;

    // Ensure no extra path segments
    if parts.next().is_some() {
        return None;
    }

    Some((owner.to_string(), repo.to_string()))
}

fn parse_github_tags(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    let tags = parsed.as_array()?;

    // First tag is typically the latest
    let first = tags.first()?;
    let name = first.get("name")?.as_str()?;

    // Strip 'v' prefix if present
    Some(name.strip_prefix('v').unwrap_or(name).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_repo_simple() {
        assert_eq!(
            parse_github_repo("apple/swift-argument-parser"),
            Some(("apple".to_string(), "swift-argument-parser".to_string()))
        );
    }

    #[test]
    fn test_parse_github_repo_full_url() {
        assert_eq!(
            parse_github_repo("https://github.com/apple/swift-argument-parser"),
            Some(("apple".to_string(), "swift-argument-parser".to_string()))
        );
    }

    #[test]
    fn test_parse_github_repo_with_git_suffix() {
        assert_eq!(
            parse_github_repo("https://github.com/apple/swift-argument-parser.git"),
            Some(("apple".to_string(), "swift-argument-parser".to_string()))
        );
    }

    #[test]
    fn test_parse_github_repo_invalid() {
        assert_eq!(parse_github_repo("invalid"), None);
        assert_eq!(parse_github_repo(""), None);
        assert_eq!(parse_github_repo("/repo"), None);
        assert_eq!(parse_github_repo("owner/"), None);
        assert_eq!(parse_github_repo("a/b/c"), None);
    }

    #[test]
    fn test_parse_github_tags() {
        let json = r#"[{"name":"1.3.0"},{"name":"1.2.0"}]"#;
        assert_eq!(parse_github_tags(json), Some("1.3.0".to_string()));
    }

    #[test]
    fn test_parse_github_tags_with_v_prefix() {
        let json = r#"[{"name":"v2.0.0"},{"name":"v1.0.0"}]"#;
        assert_eq!(parse_github_tags(json), Some("2.0.0".to_string()));
    }

    #[test]
    fn test_parse_github_tags_empty() {
        let json = r#"[]"#;
        assert_eq!(parse_github_tags(json), None);
    }

    #[test]
    fn test_swift_source_properties() {
        let swift = SwiftSource;
        assert_eq!(swift.name(), "swift");
        assert_eq!(swift.ecosystem(), Ecosystem::Swift);
        assert!(!swift.is_local());
    }
}
