use super::{Ecosystem, Source};
use std::process::Command;

pub struct NpmSource;

impl Source for NpmSource {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Npm
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let output = Command::new("npm")
            .args(["view", package, "version"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if version.is_empty() { None } else { Some(version) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_source_properties() {
        assert_eq!(NpmSource.name(), "npm");
        assert!(!NpmSource.is_local());
        assert_eq!(NpmSource.ecosystem(), Ecosystem::Npm);
    }
}
