use super::{Ecosystem, Source};
use std::process::Command;

pub struct CargoSource;

impl Source for CargoSource {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Cargo
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let output = Command::new("cargo")
            .args(["search", package, "--limit", "1"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        // Parse: package_name = "X.Y.Z"    # description
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some((name_part, rest)) = line.split_once('=') {
                if name_part.trim() == package {
                    let rest = rest.trim();
                    if rest.starts_with('"') {
                        if let Some(end) = rest[1..].find('"') {
                            return Some(rest[1..=end].to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_source_properties() {
        assert_eq!(CargoSource.name(), "cargo");
        assert!(!CargoSource.is_local());
        assert_eq!(CargoSource.ecosystem(), Ecosystem::Cargo);
    }
}
