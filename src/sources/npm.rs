use super::{Ecosystem, Source};
use std::process::Command;

pub struct NpmSource;

impl Source for NpmSource {
    fn name(&self) -> &'static str { "npm" }
    fn ecosystem(&self) -> Ecosystem { Ecosystem::Npm }

    fn get_version(&self, package: &str) -> Option<String> {
        Command::new("npm").args(["view", package, "version"]).output().ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|v| !v.is_empty())
    }
}
