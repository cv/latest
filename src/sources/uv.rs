use super::{Ecosystem, Source};
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct UvSource;

impl Source for UvSource {
    fn name(&self) -> &'static str {
        "uv"
    }

    fn is_local(&self) -> bool {
        true
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Python
    }

    fn get_version(&self, package: &str) -> Option<String> {
        if !is_uv_project() {
            return None;
        }

        // Try uv.lock first (fast, no subprocess)
        if let Some(v) = parse_uv_lock(package) {
            return Some(v);
        }

        // Fall back to uv pip show
        get_uv_installed(package)
    }
}

fn is_uv_project() -> bool {
    Path::new("uv.lock").exists()
        || (Path::new("pyproject.toml").exists() && Path::new(".venv").exists())
}

fn parse_uv_lock(package: &str) -> Option<String> {
    let content = fs::read_to_string("uv.lock").ok()?;
    
    let mut in_target = false;
    for line in content.lines() {
        if line.starts_with("[[package]]") {
            in_target = false;
        } else if let Some(name) = line.strip_prefix("name = \"").and_then(|s| s.strip_suffix('"')) {
            // Python package names are case-insensitive, normalize with - and _
            let normalized_name = name.replace('-', "_").to_lowercase();
            let normalized_pkg = package.replace('-', "_").to_lowercase();
            in_target = normalized_name == normalized_pkg;
        } else if in_target {
            if let Some(version) = line.strip_prefix("version = \"").and_then(|s| s.strip_suffix('"')) {
                return Some(version.to_string());
            }
        }
    }
    None
}

fn get_uv_installed(package: &str) -> Option<String> {
    let output = Command::new("uv").args(["pip", "show", package]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Version:").map(|v| v.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_source_properties() {
        assert_eq!(UvSource.name(), "uv");
        assert!(UvSource.is_local());
        assert_eq!(UvSource.ecosystem(), Ecosystem::Python);
    }
}
