use super::{Ecosystem, Source};
use std::process::Command;

pub struct DockerSource;

impl Source for DockerSource {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Container
    }

    fn get_version(&self, package: &str) -> Option<String> {
        // Handle official images (no slash) vs user images (user/repo)
        let repo_path = if package.contains('/') {
            package.to_string()
        } else {
            format!("library/{package}")
        };

        let url = format!(
            "https://registry.hub.docker.com/v2/repositories/{}/tags?page_size=100",
            urlencoding::encode(&repo_path).replace("%2F", "/") // Keep the slash
        );

        let output = Command::new("curl")
            .args(["-sf", "-m", "10", &url])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        parse_docker_tags(&String::from_utf8_lossy(&output.stdout))
    }
}

fn parse_docker_tags(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    let results = parsed.get("results")?.as_array()?;

    // Extract tag names, filter to version-like tags, sort semver
    let mut versions: Vec<(semver::Version, String)> = results
        .iter()
        .filter_map(|r| r.get("name")?.as_str())
        .filter_map(|tag| {
            // Try to parse as semver (handles v prefix)
            let clean = tag.strip_prefix('v').unwrap_or(tag);
            // Only consider tags that look like versions (start with digit)
            if !clean.chars().next()?.is_ascii_digit() {
                return None;
            }
            // Try to parse, padding with .0 if needed
            let padded = pad_version(clean);
            semver::Version::parse(&padded)
                .ok()
                .map(|v| (v, tag.to_string()))
        })
        .collect();

    versions.sort_by(|a, b| b.0.cmp(&a.0)); // Descending
    versions.first().map(|(_, tag)| tag.clone())
}

fn pad_version(v: &str) -> String {
    // Split on first non-version char (like -alpine, -slim)
    let base = v
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()
        .unwrap_or(v);
    let parts: Vec<&str> = base.split('.').collect();
    match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => base.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_version() {
        assert_eq!(pad_version("3"), "3.0.0");
        assert_eq!(pad_version("3.21"), "3.21.0");
        assert_eq!(pad_version("3.21.0"), "3.21.0");
        assert_eq!(pad_version("3.21-alpine"), "3.21.0");
    }

    #[test]
    fn test_parse_docker_tags() {
        let json =
            r#"{"results":[{"name":"latest"},{"name":"3.21"},{"name":"3.20"},{"name":"alpine"}]}"#;
        assert_eq!(parse_docker_tags(json), Some("3.21".to_string()));
    }

    #[test]
    fn test_parse_docker_tags_with_v_prefix() {
        let json = r#"{"results":[{"name":"v1.0.0"},{"name":"v0.9.0"}]}"#;
        assert_eq!(parse_docker_tags(json), Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_parse_docker_tags_empty() {
        let json = r#"{"results":[]}"#;
        assert_eq!(parse_docker_tags(json), None);
    }

    #[test]
    fn test_parse_docker_tags_no_versions() {
        let json = r#"{"results":[{"name":"latest"},{"name":"alpine"}]}"#;
        assert_eq!(parse_docker_tags(json), None);
    }

    #[test]
    fn test_docker_source_properties() {
        let docker = DockerSource;
        assert_eq!(docker.name(), "docker");
        assert_eq!(docker.ecosystem(), Ecosystem::Container);
        assert!(!docker.is_local());
    }
}
