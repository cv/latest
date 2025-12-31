//! Simple file-based cache for registry responses

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_TTL_SECS: u64 = 3600; // 1 hour

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    version: String,
    timestamp: u64,
}

/// Get the cache directory (~/.cache/latest/)
fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|p| p.join("latest"))
}

/// Get cached version if valid (not expired)
pub fn get(source: &str, package: &str) -> Option<String> {
    let path = cache_dir()?.join(format!("{}-{}.json", source, sanitize(package)));
    let content = fs::read_to_string(&path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&content).ok()?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    if now - entry.timestamp < DEFAULT_TTL_SECS {
        Some(entry.version)
    } else {
        // Expired - remove stale cache file
        let _ = fs::remove_file(&path);
        None
    }
}

/// Store version in cache
pub fn set(source: &str, package: &str, version: &str) {
    let Some(dir) = cache_dir() else { return };
    let _ = fs::create_dir_all(&dir);

    let path = dir.join(format!("{}-{}.json", source, sanitize(package)));
    let entry = CacheEntry {
        version: version.to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    };

    if let Ok(content) = serde_json::to_string(&entry) {
        let _ = fs::write(&path, content);
    }
}

/// Sanitize package name for use in filename
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize("express"), "express");
        assert_eq!(sanitize("@babel/core"), "_babel_core");
        assert_eq!(sanitize("github.com/spf13/cobra"), "github_com_spf13_cobra");
    }
}
