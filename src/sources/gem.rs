use super::{Ecosystem, Source};
use std::process::Command;

pub struct GemSource;

impl Source for GemSource {
    fn name(&self) -> &'static str { "gem" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::Ruby }

    fn get_version(&self, package: &str) -> Option<String> {
        // gem search ^name$ --remote returns "name (version)"
        let output = Command::new("gem")
            .args(["search", &format!("^{}$", package), "--remote"])
            .output().ok()?;
        if !output.status.success() { return None; }

        String::from_utf8_lossy(&output.stdout).lines()
            .find_map(|line| {
                let line = line.trim();
                if !line.starts_with(package) { return None; }
                line.split('(').nth(1)?.split(')').next().map(|v| v.to_string())
            })
    }
}
