// Library crate exposing internals for benchmarking

pub mod cache;
pub mod config;
pub mod project;
pub mod sources;

/// Check if `latest` is a newer version than `installed`.
/// Compares numeric version components.
pub fn is_newer(installed: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit()).filter_map(|s| s.parse().ok()).collect()
    };
    let (a, b) = (parse(installed), parse(latest));
    (0..a.len().max(b.len())).any(|i| {
        let (x, y) = (*a.get(i).unwrap_or(&0), *b.get(i).unwrap_or(&0));
        x < y && (0..i).all(|j| a.get(j) == b.get(j))
    })
}

/// Parse a package argument, extracting optional source prefix.
/// e.g., "npm:express" -> (Some("npm"), "express")
///       "express" -> (None, "express")
pub fn parse_package_arg(arg: &str) -> (Option<String>, String) {
    if let Some((prefix, rest)) = arg.split_once(':') {
        // Only treat as source prefix if it's a known source name
        if sources::source_by_name(prefix).is_some() {
            return (Some(prefix.to_string()), rest.to_string());
        }
    }
    (None, arg.to_string())
}
