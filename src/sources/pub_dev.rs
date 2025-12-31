use super::{Ecosystem, Source};
use std::process::Command;

pub struct PubSource;

impl Source for PubSource {
    fn name(&self) -> &'static str { "pub" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::Dart }

    fn get_version(&self, package: &str) -> Option<String> {
        // Use curl to query pub.dev API
        let output = Command::new("curl")
            .args(["-sf", &format!("https://pub.dev/api/packages/{}", package)])
            .output().ok()?;
        if !output.status.success() { return None; }

        // Parse "latest":{"version":"X.Y.Z" from JSON
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.split("\"latest\":{\"version\":\"").nth(1)?
            .split('"').next()
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
    }
}
