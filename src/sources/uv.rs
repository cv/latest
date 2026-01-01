use super::{Ecosystem, Source, extract_version_field};
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
        let is_uv_project = Path::new("uv.lock").exists()
            || (Path::new("pyproject.toml").exists() && Path::new(".venv").exists());
        if !is_uv_project {
            return None;
        }

        // Try uv.lock first (fast, no subprocess), then uv pip show
        parse_uv_lock(package).or_else(|| {
            Command::new("uv")
                .args(["pip", "show", package])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| extract_version_field(&String::from_utf8_lossy(&o.stdout)))
        })
    }
}

#[allow(clippy::collapsible_if)] // Let chains require nightly rustfmt
fn parse_uv_lock(package: &str) -> Option<String> {
    let content = fs::read_to_string("uv.lock").ok()?;
    let normalized_pkg = package.replace('-', "_").to_lowercase();

    let mut in_target = false;
    for line in content.lines() {
        if line.starts_with("[[package]]") {
            in_target = false;
        } else if let Some(name) = line.strip_prefix("name = \"").and_then(|s| s.strip_suffix('"'))
        {
            in_target = name.replace('-', "_").to_lowercase() == normalized_pkg;
        } else if in_target {
            if let Some(v) = line.strip_prefix("version = \"").and_then(|s| s.strip_suffix('"')) {
                return Some(v.to_string());
            }
        }
    }
    None
}
