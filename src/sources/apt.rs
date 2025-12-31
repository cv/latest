use super::{extract_version_field, Ecosystem, Source};
use std::process::Command;

pub struct AptSource;

impl Source for AptSource {
    fn name(&self) -> &'static str { "apt" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::System }

    fn get_version(&self, package: &str) -> Option<String> {
        // Check if apt-cache is available
        Command::new("which").arg("apt-cache").output().ok().filter(|o| o.status.success())?;
        
        let output = Command::new("apt-cache").args(["show", package]).output().ok()?;
        if !output.status.success() { return None; }
        
        extract_version_field(&String::from_utf8_lossy(&output.stdout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apt_output() {
        let output = r#"Package: curl
Version: 7.88.1-10+deb12u5
Priority: optional
Section: web
Maintainer: Alessandro Ghedini <ghedo@debian.org>
Installed-Size: 518
"#;
        assert_eq!(extract_version_field(output), Some("7.88.1-10+deb12u5".to_string()));
    }

    #[test]
    fn test_parse_apt_output_no_version() {
        let output = "Package: something\nPriority: optional\n";
        assert_eq!(extract_version_field(output), None);
    }

    #[test]
    fn test_parse_apt_output_empty() {
        assert_eq!(extract_version_field(""), None);
    }

    #[test]
    fn test_apt_source_properties() {
        let apt = AptSource;
        assert_eq!(apt.name(), "apt");
        assert_eq!(apt.ecosystem(), Ecosystem::System);
        assert!(!apt.is_local());
    }
}
