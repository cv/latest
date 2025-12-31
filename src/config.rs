use crate::sources::{default_precedence, SourceType};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_precedence")]
    pub precedence: Vec<SourceType>,
}

impl Default for Config {
    fn default() -> Self {
        Self { precedence: default_precedence() }
    }
}

impl Config {
    pub fn load() -> Self {
        dirs::config_dir()
            .map(|p| p.join("latest/config.toml"))
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_has_all_sources() {
        let config = Config::default();
        assert_eq!(config.precedence.len(), 11);
    }

    #[test]
    fn test_parse_config() {
        let config: Config = toml::from_str(r#"precedence = ["npm", "cargo"]"#).unwrap();
        assert_eq!(config.precedence.len(), 2);
    }
}
