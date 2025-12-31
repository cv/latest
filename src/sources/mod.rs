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

pub trait Source {
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
    let re = regex::Regex::new(r"v?(\d+\.\d+(?:\.\d+)?(?:-[a-zA-Z0-9.-]+)?)").ok()?;
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_standard() {
        assert_eq!(extract_version("1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(extract_version("v1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(extract_version("1.2"), Some("1.2".to_string()));
    }

    #[test]
    fn test_extract_version_real_world() {
        assert_eq!(extract_version("go version go1.25.5 darwin/arm64"), Some("1.25.5".to_string()));
        assert_eq!(extract_version("node v20.10.0"), Some("20.10.0".to_string()));
        assert_eq!(extract_version("Python 3.12.1"), Some("3.12.1".to_string()));
    }

    #[test]
    fn test_extract_version_none() {
        assert_eq!(extract_version("no version here"), None);
        assert_eq!(extract_version(""), None);
    }

    #[test]
    fn test_source_type_from_name() {
        assert_eq!(SourceType::from_name("npm"), Some(SourceType::Npm));
        assert_eq!(SourceType::from_name("cargo"), Some(SourceType::Cargo));
        assert_eq!(SourceType::from_name("invalid"), None);
    }

    #[test]
    fn test_source_type_create() {
        let source = SourceType::Npm.create();
        assert_eq!(source.name(), "npm");
        assert_eq!(source.ecosystem(), Ecosystem::Npm);
    }

    #[test]
    fn test_all_sources_have_consistent_ecosystem() {
        // System ecosystem
        assert_eq!(PathSource.ecosystem(), Ecosystem::System);
        assert_eq!(BrewSource.ecosystem(), Ecosystem::System);
        
        // Python ecosystem
        assert_eq!(PipSource.ecosystem(), Ecosystem::Python);
        assert_eq!(UvSource.ecosystem(), Ecosystem::Python);
        
        // Individual ecosystems
        assert_eq!(NpmSource.ecosystem(), Ecosystem::Npm);
        assert_eq!(CargoSource.ecosystem(), Ecosystem::Cargo);
        assert_eq!(GoSource.ecosystem(), Ecosystem::Go);
    }
}
