use super::{extract_version, Ecosystem, Source};
use std::process::Command;

pub struct PipSource;

impl Source for PipSource {
    fn name(&self) -> &'static str {
        "pip"
    }

    fn is_local(&self) -> bool {
        true  // pip checks local installs first, then falls back to PyPI
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Python
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let pip = find_pip()?;

        // Try local install first
        if let Some(v) = get_installed_version(pip, package) {
            return Some(v);
        }

        // Fall back to PyPI
        get_pypi_version(pip, package)
    }
}

fn find_pip() -> Option<&'static str> {
    for cmd in ["pip", "pip3"] {
        if Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false) {
            return Some(cmd);
        }
    }
    None
}

fn get_installed_version(pip: &str, package: &str) -> Option<String> {
    let output = Command::new(pip).args(["show", package]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Version:").map(|v| v.trim().to_string()))
}

fn get_pypi_version(pip: &str, package: &str) -> Option<String> {
    let output = Command::new(pip).args(["index", "versions", package]).output().ok()?;
    if output.status.success() {
        extract_version(&String::from_utf8_lossy(&output.stdout))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_source_properties() {
        assert_eq!(PipSource.name(), "pip");
        assert!(PipSource.is_local());
        assert_eq!(PipSource.ecosystem(), Ecosystem::Python);
    }
}
