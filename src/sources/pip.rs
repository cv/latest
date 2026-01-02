use super::{Ecosystem, Source, extract_version_field};
use std::process::Command;

pub struct PipSource;

impl Source for PipSource {
    fn name(&self) -> &'static str {
        "pip"
    }
    fn is_local(&self) -> bool {
        true
    }
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Python
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let pip = ["pip", "pip3"].into_iter().find(|cmd| {
            Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
        })?;

        // Only check locally installed packages
        Command::new(pip)
            .args(["show", package])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| extract_version_field(&String::from_utf8_lossy(&o.stdout)))
    }
}
