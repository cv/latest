mod path;
mod brew;
mod npm;
mod pip;
mod go;
mod cargo;
mod uv;

pub use path::PathSource;
pub use brew::BrewSource;
pub use npm::NpmSource;
pub use pip::PipSource;
pub use go::GoSource;
pub use cargo::CargoSource;
pub use uv::UvSource;

use serde::Deserialize;
use std::sync::LazyLock;

static VERSION_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"v?(\d+\.\d+(?:\.\d+)?(?:-[a-zA-Z0-9.-]+)?)").unwrap()
});

/// Ecosystem grouping - sources in the same ecosystem can be version-compared
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ecosystem {
    System,  // path, brew - system-level binaries
    Python,  // uv, pip - Python packages  
    Npm,     // npm - Node packages
    Cargo,   // cargo - Rust crates
    Go,      // go - Go modules
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Path,
    Brew,
    Npm,
    Pip,
    Go,
    Cargo,
    Uv,
}

impl SourceType {
    /// Create a boxed Source from this type
    pub fn create(&self) -> Box<dyn Source> {
        match self {
            SourceType::Path => Box::new(PathSource),
            SourceType::Brew => Box::new(BrewSource),
            SourceType::Npm => Box::new(NpmSource),
            SourceType::Pip => Box::new(PipSource),
            SourceType::Go => Box::new(GoSource),
            SourceType::Cargo => Box::new(CargoSource),
            SourceType::Uv => Box::new(UvSource),
        }
    }

    /// Parse a source name string
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "path" => Some(SourceType::Path),
            "brew" => Some(SourceType::Brew),
            "npm" => Some(SourceType::Npm),
            "pip" => Some(SourceType::Pip),
            "go" => Some(SourceType::Go),
            "cargo" => Some(SourceType::Cargo),
            "uv" => Some(SourceType::Uv),
            _ => None,
        }
    }
}

pub trait Source: Send + Sync {
    fn name(&self) -> &'static str;
    fn get_version(&self, package: &str) -> Option<String>;
    
    /// Returns true if this source checks locally installed packages
    fn is_local(&self) -> bool {
        false
    }

    /// Which ecosystem this source belongs to
    fn ecosystem(&self) -> Ecosystem;
}

/// Extract a semver-like version from text
pub fn extract_version(text: &str) -> Option<String> {
    VERSION_REGEX
        .captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract version from "Version: X.Y.Z" line (used by pip/uv)
pub fn extract_version_field(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("Version:").map(|v| v.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version() {
        // Standard formats
        assert_eq!(extract_version("1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(extract_version("v1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(extract_version("1.2"), Some("1.2".to_string()));
        // Real-world
        assert_eq!(extract_version("go version go1.25.5 darwin/arm64"), Some("1.25.5".to_string()));
        assert_eq!(extract_version("node v20.10.0"), Some("20.10.0".to_string()));
        // None
        assert_eq!(extract_version("no version here"), None);
        assert_eq!(extract_version(""), None);
    }

    #[test]
    fn test_extract_version_field() {
        assert_eq!(extract_version_field("Name: foo\nVersion: 1.2.3\n"), Some("1.2.3".to_string()));
        assert_eq!(extract_version_field("no version here"), None);
    }

    #[test]
    fn test_source_type_from_name() {
        for (name, expected) in [("npm", SourceType::Npm), ("cargo", SourceType::Cargo), ("path", SourceType::Path)] {
            assert_eq!(SourceType::from_name(name), Some(expected));
        }
        assert_eq!(SourceType::from_name("invalid"), None);
    }

    #[test]
    fn test_all_sources() {
        let cases: Vec<(Box<dyn Source>, &str, bool, Ecosystem)> = vec![
            (Box::new(PathSource), "path", true, Ecosystem::System),
            (Box::new(BrewSource), "brew", false, Ecosystem::System),
            (Box::new(NpmSource), "npm", false, Ecosystem::Npm),
            (Box::new(PipSource), "pip", true, Ecosystem::Python),
            (Box::new(CargoSource), "cargo", false, Ecosystem::Cargo),
            (Box::new(GoSource), "go", false, Ecosystem::Go),
            (Box::new(UvSource), "uv", true, Ecosystem::Python),
        ];
        for (source, name, local, ecosystem) in cases {
            assert_eq!(source.name(), name, "name mismatch for {}", name);
            assert_eq!(source.is_local(), local, "is_local mismatch for {}", name);
            assert_eq!(source.ecosystem(), ecosystem, "ecosystem mismatch for {}", name);
        }
    }
}
