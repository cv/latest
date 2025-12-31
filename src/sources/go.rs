use super::{Ecosystem, Source};
use std::process::Command;

pub struct GoSource;

impl Source for GoSource {
    fn name(&self) -> &'static str { "go" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::Go }

    fn get_version(&self, package: &str) -> Option<String> {
        Command::new("go").args(["list", "-m", "-versions", package]).output().ok()
            .filter(|o| o.status.success())
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout).split_whitespace().last()
                    .filter(|v| v.starts_with('v'))
                    .map(|v| v.strip_prefix('v').unwrap_or(v).to_string())
            })
    }
}
