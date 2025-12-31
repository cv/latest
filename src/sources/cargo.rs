use super::{Ecosystem, Source};
use std::process::Command;

pub struct CargoSource;

impl Source for CargoSource {
    fn name(&self) -> &'static str { "cargo" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::Cargo }

    fn get_version(&self, package: &str) -> Option<String> {
        let output = Command::new("cargo").args(["search", package, "--limit", "1"]).output().ok()?;
        if !output.status.success() { return None; }

        // Parse: package_name = "X.Y.Z"    # description
        String::from_utf8_lossy(&output.stdout).lines()
            .find_map(|line| {
                let (name, rest) = line.split_once('=')?;
                if name.trim() != package { return None; }
                let rest = rest.trim().strip_prefix('"')?;
                Some(rest[..rest.find('"')?].to_string())
            })
    }
}
