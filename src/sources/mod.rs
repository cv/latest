mod apt;
mod brew;
mod path;
mod pip;
mod uv;

use serde::Deserialize;
use std::process::Command;
use std::sync::LazyLock;

static VERSION_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    // This regex is a compile-time constant and will never fail
    // Use [0-9] instead of \d to avoid matching Unicode digits (e.g., ೦)
    #[allow(clippy::unwrap_used)]
    regex::Regex::new(r"v?([0-9]+\.[0-9]+(?:\.[0-9]+)?(?:-[a-zA-Z0-9.-]+)?)").unwrap()
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecosystem {
    System,
    Python,
    Npm,
    Cargo,
    Go,
    Ruby,
    Beam,
    Dart,
}

pub trait Source: Send + Sync {
    fn name(&self) -> &'static str;
    fn get_version(&self, package: &str) -> Option<String>;
    fn is_local(&self) -> bool {
        false
    }
    fn ecosystem(&self) -> Ecosystem;
}

#[must_use]
pub fn extract_version(text: &str) -> Option<String> {
    VERSION_REGEX.captures(text).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

#[must_use]
pub fn extract_version_field(text: &str) -> Option<String> {
    text.lines().find_map(|l| l.strip_prefix("Version:").map(|v| v.trim().to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON API source - for registries with HTTP JSON APIs
// ─────────────────────────────────────────────────────────────────────────────

struct JsonApiSource {
    name: &'static str,
    ecosystem: Ecosystem,
    url_template: &'static str,
    version_path: &'static str,
}

impl JsonApiSource {
    fn build_url(&self, package: &str) -> String {
        let encoded_package = urlencoding::encode(package);
        self.url_template.replace("{}", &encoded_package)
    }

    fn fetch(&self, package: &str) -> Option<String> {
        let url = self.build_url(package);
        let output = Command::new("curl").args(["-sf", &url]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let version =
            extract_json_path(&String::from_utf8_lossy(&output.stdout), self.version_path)?;
        Some(version.strip_prefix('v').unwrap_or(&version).to_string())
    }
}

impl Source for &'static JsonApiSource {
    fn name(&self) -> &'static str {
        self.name
    }
    fn ecosystem(&self) -> Ecosystem {
        self.ecosystem
    }
    fn get_version(&self, package: &str) -> Option<String> {
        self.fetch(package)
    }
}

fn extract_json_path(json: &str, path: &str) -> Option<String> {
    let mut current = json;
    for key in path.split('.') {
        current = current.split(&format!("\"{key}\":")).nth(1)?;
    }
    let start = current.find('"')? + 1;
    let rest = &current[start..];
    Some(rest[..rest.find('"')?].to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Source registry - define all sources in ONE place
// ─────────────────────────────────────────────────────────────────────────────

pub use apt::AptSource;
pub use brew::BrewSource;
pub use path::PathSource;
pub use pip::PipSource;
pub use uv::UvSource;

// JSON API sources - no CLI needed, just HTTP
static NPM: JsonApiSource = JsonApiSource {
    name: "npm",
    ecosystem: Ecosystem::Npm,
    url_template: "https://registry.npmjs.org/{}/latest",
    version_path: "version",
};
static CARGO: JsonApiSource = JsonApiSource {
    name: "cargo",
    ecosystem: Ecosystem::Cargo,
    url_template: "https://crates.io/api/v1/crates/{}",
    version_path: "crate.max_stable_version",
};
static GO: JsonApiSource = JsonApiSource {
    name: "go",
    ecosystem: Ecosystem::Go,
    url_template: "https://proxy.golang.org/{}/@latest",
    version_path: "Version",
};
static GEM: JsonApiSource = JsonApiSource {
    name: "gem",
    ecosystem: Ecosystem::Ruby,
    url_template: "https://rubygems.org/api/v1/gems/{}.json",
    version_path: "version",
};
static HEX: JsonApiSource = JsonApiSource {
    name: "hex",
    ecosystem: Ecosystem::Beam,
    url_template: "https://hex.pm/api/packages/{}",
    version_path: "latest_stable_version",
};
static PUB: JsonApiSource = JsonApiSource {
    name: "pub",
    ecosystem: Ecosystem::Dart,
    url_template: "https://pub.dev/api/packages/{}",
    version_path: "latest.version",
};

/// Source definitions: (name, `type_variant`, constructor, `is_local`, ecosystem)
/// This is the SINGLE source of truth.
macro_rules! define_sources {
    ($($name:literal, $variant:ident => $create:expr, $local:literal, $eco:expr);* $(;)?) => {
        #[allow(dead_code)]
        pub fn all_sources() -> Vec<Box<dyn Source>> {
            vec![$(Box::new($create)),*]
        }

        pub fn source_by_name(name: &str) -> Option<Box<dyn Source>> {
            match name { $($name => Some(Box::new($create)),)* _ => None }
        }

        #[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "lowercase")]
        pub enum SourceType { $($variant),* }

        impl SourceType {
            #[allow(clippy::unwrap_used)]
            pub fn create(&self) -> Box<dyn Source> { source_by_name(self.as_str()).unwrap() }
            pub const fn as_str(&self) -> &'static str {
                match self { $(SourceType::$variant => $name),* }
            }
        }

        pub fn default_precedence() -> Vec<SourceType> {
            vec![$(SourceType::$variant),*]
        }

        #[cfg(test)]
        fn expected_sources() -> Vec<(&'static str, bool, Ecosystem)> {
            vec![$(($name, $local, $eco)),*]
        }
    };
}

define_sources! {
    "path",  Path  => PathSource,  true,  Ecosystem::System;
    "brew",  Brew  => BrewSource,  false, Ecosystem::System;
    "apt",   Apt   => AptSource,   false, Ecosystem::System;
    "npm",   Npm   => &NPM,        false, Ecosystem::Npm;
    "uv",    Uv    => UvSource,    true,  Ecosystem::Python;
    "pip",   Pip   => PipSource,   true,  Ecosystem::Python;
    "go",    Go    => &GO,         false, Ecosystem::Go;
    "cargo", Cargo => &CARGO,      false, Ecosystem::Cargo;
    "gem",   Gem   => &GEM,        false, Ecosystem::Ruby;
    "hex",   Hex   => &HEX,        false, Ecosystem::Beam;
    "pub",   Pub   => &PUB,        false, Ecosystem::Dart;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version("1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(extract_version("v1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(extract_version(""), None);
    }

    #[test]
    fn test_extract_json_path() {
        assert_eq!(
            extract_json_path(r#"{"version":"1.2.3"}"#, "version"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            extract_json_path(r#"{"latest":{"version":"2.0"}}"#, "latest.version"),
            Some("2.0".to_string())
        );
    }

    #[test]
    fn test_all_sources() {
        let sources = all_sources();
        let expected = expected_sources();
        assert_eq!(sources.len(), expected.len());
        for (source, (name, local, eco)) in sources.iter().zip(expected.iter()) {
            assert_eq!(source.name(), *name);
            assert_eq!(source.is_local(), *local);
            assert_eq!(source.ecosystem(), *eco);
        }
    }

    #[test]
    fn test_source_by_name() {
        for (name, _, _) in expected_sources() {
            assert!(source_by_name(name).is_some(), "missing: {}", name);
        }
        assert!(source_by_name("invalid").is_none());
    }

    #[test]
    fn test_url_encoding_normal_package() {
        let source = JsonApiSource {
            name: "test",
            ecosystem: Ecosystem::Npm,
            url_template: "https://example.com/{}",
            version_path: "version",
        };
        assert_eq!(source.build_url("express"), "https://example.com/express");
        assert_eq!(source.build_url("lodash"), "https://example.com/lodash");
    }

    #[test]
    fn test_url_encoding_special_characters() {
        let source = JsonApiSource {
            name: "test",
            ecosystem: Ecosystem::Npm,
            url_template: "https://example.com/{}",
            version_path: "version",
        };
        // Path traversal attempt should be encoded
        assert_eq!(
            source.build_url("../../api/v1/users"),
            "https://example.com/..%2F..%2Fapi%2Fv1%2Fusers"
        );
        // Query injection should be encoded
        assert_eq!(
            source.build_url("express?malicious=true"),
            "https://example.com/express%3Fmalicious%3Dtrue"
        );
        // Fragment injection should be encoded
        assert_eq!(source.build_url("pkg#anchor"), "https://example.com/pkg%23anchor");
        // Ampersand should be encoded
        assert_eq!(source.build_url("a&b=c"), "https://example.com/a%26b%3Dc");
        // Spaces should be encoded
        assert_eq!(source.build_url("my package"), "https://example.com/my%20package");
    }

    #[test]
    fn test_url_encoding_unicode() {
        let source = JsonApiSource {
            name: "test",
            ecosystem: Ecosystem::Npm,
            url_template: "https://example.com/{}",
            version_path: "version",
        };
        // Unicode should be percent-encoded
        assert_eq!(source.build_url("日本語"), "https://example.com/%E6%97%A5%E6%9C%AC%E8%AA%9E");
    }

    #[test]
    fn test_url_encoding_scoped_npm_packages() {
        let source = JsonApiSource {
            name: "test",
            ecosystem: Ecosystem::Npm,
            url_template: "https://example.com/{}",
            version_path: "version",
        };
        // @ and scoped package names - @ gets encoded
        assert_eq!(source.build_url("@scope/pkg"), "https://example.com/%40scope%2Fpkg");
    }
}
