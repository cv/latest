use super::{Ecosystem, Source};
use std::process::Command;

pub struct MavenSource;

impl Source for MavenSource {
    fn name(&self) -> &'static str {
        "maven"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Jvm
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let (group, artifact) = parse_maven_coordinates(package)?;
        let url = format!(
            "https://search.maven.org/solrsearch/select?q=g:{}+AND+a:{}&rows=1&wt=json",
            urlencoding::encode(group),
            urlencoding::encode(artifact)
        );
        let output = Command::new("curl").args(["-sf", "-m", "10", &url]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        parse_maven_response(&String::from_utf8_lossy(&output.stdout))
    }
}

fn parse_maven_coordinates(package: &str) -> Option<(&str, &str)> {
    let mut parts = package.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    // Ensure no extra parts
    if parts.next().is_some() {
        return None;
    }
    // Validate non-empty
    if group.is_empty() || artifact.is_empty() {
        return None;
    }
    Some((group, artifact))
}

fn parse_maven_response(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    let docs = parsed.get("response")?.get("docs")?.as_array()?;
    let first = docs.first()?;
    first.get("latestVersion")?.as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_maven_coordinates() {
        assert_eq!(
            parse_maven_coordinates("org.springframework:spring-core"),
            Some(("org.springframework", "spring-core"))
        );
        assert_eq!(
            parse_maven_coordinates("com.google.guava:guava"),
            Some(("com.google.guava", "guava"))
        );
    }

    #[test]
    fn test_parse_maven_coordinates_invalid() {
        assert_eq!(parse_maven_coordinates("invalid"), None);
        assert_eq!(parse_maven_coordinates(""), None);
        assert_eq!(parse_maven_coordinates(":artifact"), None);
        assert_eq!(parse_maven_coordinates("group:"), None);
        assert_eq!(parse_maven_coordinates("a:b:c"), None);
    }

    #[test]
    fn test_parse_maven_response() {
        let json = r#"{"response":{"docs":[{"latestVersion":"5.3.30"}]}}"#;
        assert_eq!(parse_maven_response(json), Some("5.3.30".to_string()));
    }

    #[test]
    fn test_parse_maven_response_empty() {
        let json = r#"{"response":{"docs":[]}}"#;
        assert_eq!(parse_maven_response(json), None);
    }

    #[test]
    fn test_maven_source_properties() {
        let maven = MavenSource;
        assert_eq!(maven.name(), "maven");
        assert_eq!(maven.ecosystem(), Ecosystem::Jvm);
        assert!(!maven.is_local());
    }
}
