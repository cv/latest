//! Project file scanning - detects and parses dependency files

use std::fs;

pub struct ProjectInfo {
    pub file: &'static str,
    pub source: &'static str, // Which source to use: "cargo", "npm", "pip", "go"
    pub packages: Vec<String>,
}

/// Scan current directory for project files
pub fn scan() -> Option<ProjectInfo> {
    scan_cargo().or_else(scan_npm).or_else(scan_uv).or_else(scan_pyproject).or_else(scan_go)
}

fn scan_cargo() -> Option<ProjectInfo> {
    let content = fs::read_to_string("Cargo.toml").ok()?;
    let doc: toml::Value = toml::from_str(&content).ok()?;

    let mut packages = Vec::new();

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = doc.get(section).and_then(|d| d.as_table()) {
            packages.extend(deps.keys().cloned());
        }
    }

    if packages.is_empty() {
        return None;
    }

    Some(ProjectInfo { file: "Cargo.toml", source: "cargo", packages })
}

fn scan_npm() -> Option<ProjectInfo> {
    let content = fs::read_to_string("package.json").ok()?;
    let doc: serde_json::Value = serde_json::from_str(&content).ok()?;

    let mut packages = Vec::new();

    for section in ["dependencies", "devDependencies"] {
        if let Some(deps) = doc.get(section).and_then(|d| d.as_object()) {
            packages.extend(deps.keys().cloned());
        }
    }

    if packages.is_empty() {
        return None;
    }

    Some(ProjectInfo { file: "package.json", source: "npm", packages })
}

fn scan_uv() -> Option<ProjectInfo> {
    let content = fs::read_to_string("uv.lock").ok()?;

    let packages: Vec<String> = content
        .lines()
        .filter_map(|line| {
            line.strip_prefix("name = \"").and_then(|s| s.strip_suffix('"')).map(str::to_string)
        })
        .collect();

    if packages.is_empty() {
        return None;
    }

    Some(ProjectInfo { file: "uv.lock", source: "pip", packages })
}

fn scan_pyproject() -> Option<ProjectInfo> {
    let content = fs::read_to_string("pyproject.toml").ok()?;
    let doc: toml::Value = toml::from_str(&content).ok()?;

    let deps = doc.get("project")?.get("dependencies")?.as_array()?;

    let packages: Vec<String> = deps
        .iter()
        .filter_map(|d| d.as_str())
        .map(|s| {
            // Parse "flask>=3.0" -> "flask"
            s.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .next()
                .unwrap_or(s)
                .to_string()
        })
        .collect();

    if packages.is_empty() {
        return None;
    }

    Some(ProjectInfo { file: "pyproject.toml", source: "pip", packages })
}

#[allow(clippy::collapsible_if)] // Let chains require nightly rustfmt
fn scan_go() -> Option<ProjectInfo> {
    let content = fs::read_to_string("go.mod").ok()?;

    let mut packages = Vec::new();
    let mut in_require = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("require (") {
            in_require = true;
        } else if line == ")" {
            in_require = false;
        } else if line.starts_with("require ") {
            if let Some(pkg) =
                line.strip_prefix("require ").and_then(|s| s.split_whitespace().next())
            {
                packages.push(pkg.to_string());
            }
        } else if in_require && !line.is_empty() && !line.starts_with("//") {
            if let Some(pkg) = line.split_whitespace().next() {
                packages.push(pkg.to_string());
            }
        }
    }

    if packages.is_empty() {
        return None;
    }

    Some(ProjectInfo { file: "go.mod", source: "go", packages })
}
