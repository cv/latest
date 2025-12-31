use super::{extract_version, Ecosystem, Source};
use std::process::Command;

pub struct PathSource;

impl Source for PathSource {
    fn name(&self) -> &'static str {
        "path"
    }

    fn is_local(&self) -> bool {
        true
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::System
    }

    fn get_version(&self, package: &str) -> Option<String> {
        // Check if the command exists in PATH
        let which = Command::new("which").arg(package).output().ok()?;
        if !which.status.success() {
            return None;
        }

        // Try common version flags
        for flag in ["--version", "-version", "version", "-V", "-v"] {
            if let Ok(output) = Command::new(package).arg(flag).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                if let Some(v) = extract_version(&stdout).or_else(|| extract_version(&stderr)) {
                    return Some(v);
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
    fn test_path_source_properties() {
        assert_eq!(PathSource.name(), "path");
        assert!(PathSource.is_local());
        assert_eq!(PathSource.ecosystem(), Ecosystem::System);
    }
}
