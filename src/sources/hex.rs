use super::{Ecosystem, Source};
use std::process::Command;

pub struct HexSource;

impl Source for HexSource {
    fn name(&self) -> &'static str { "hex" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::Beam }

    fn get_version(&self, package: &str) -> Option<String> {
        // Use curl to query hex.pm API
        let output = Command::new("curl")
            .args(["-sf", &format!("https://hex.pm/api/packages/{}", package)])
            .output().ok()?;
        if !output.status.success() { return None; }

        // Parse "latest_stable_version":"X.Y.Z" from JSON
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.split("\"latest_stable_version\":\"").nth(1)?
            .split('"').next()
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
    }
}
