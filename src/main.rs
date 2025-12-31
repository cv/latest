mod cache;
mod config;
mod project;
mod sources;

use clap::Parser;
use config::Config;
use rayon::prelude::*;
use sources::{Source, SourceType};

#[derive(Parser)]
#[command(name = "latest")]
#[command(about = "Find the latest version of any command, package, or library")]
struct Cli {
    /// Packages to look up (if empty, scans project files)
    packages: Vec<String>,

    /// Only check a specific source (path, brew, npm, pip, go, cargo, uv)
    #[arg(short, long)]
    source: Option<String>,

    /// Show all sources where the package is found
    #[arg(short, long)]
    all: bool,

    /// Output as JSON
    #[arg(short, long)]
    json: bool,

    /// Only show version number
    #[arg(short, long)]
    quiet: bool,

    /// Bypass cache (always fetch fresh data)
    #[arg(long)]
    no_cache: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Result types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Status {
    UpToDate,
    Outdated,
    NotInstalled,
    NotFound,
}

#[derive(serde::Serialize, Clone)]
struct VersionInfo {
    version: String,
    source: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    local: bool,
}

#[derive(serde::Serialize)]
struct PackageResult {
    package: String,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed: Option<VersionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest: Option<VersionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    available: Vec<VersionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    install_commands: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────────────────────

fn is_newer(installed: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    
    let (a, b) = (parse(installed), parse(latest));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).unwrap_or(&0), b.get(i).unwrap_or(&0));
        match x.cmp(y) {
            std::cmp::Ordering::Less => return true,
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => continue,
        }
    }
    false
}

fn lookup(package: &str, sources: &[Box<dyn Source>], mode: LookupMode, use_cache: bool) -> PackageResult {
    match mode {
        LookupMode::All => lookup_all(package, sources, use_cache),
        LookupMode::Explicit => lookup_explicit(package, sources, use_cache),
        LookupMode::Default => lookup_default(package, sources, use_cache),
    }
}

#[derive(Clone, Copy)]
enum LookupMode {
    All,      // --all flag
    Explicit, // -s flag
    Default,  // normal behavior
}

/// Query a source with optional caching (only for non-local sources)
fn query_source(source: &Box<dyn Source>, package: &str, use_cache: bool) -> Option<String> {
    // Local sources are never cached (they check installed versions)
    if source.is_local() {
        return source.get_version(package);
    }

    // Try cache first
    if use_cache {
        if let Some(cached) = cache::get(source.name(), package) {
            return Some(cached);
        }
    }

    // Query source and cache result
    let version = source.get_version(package)?;
    if use_cache {
        cache::set(source.name(), package, &version);
    }
    Some(version)
}

fn lookup_all(package: &str, sources: &[Box<dyn Source>], use_cache: bool) -> PackageResult {
    let available: Vec<_> = sources
        .par_iter()
        .filter_map(|s| {
            query_source(s, package, use_cache).map(|v| VersionInfo {
                version: v,
                source: s.name().to_string(),
                local: s.is_local(),
            })
        })
        .collect();

    PackageResult {
        package: package.to_string(),
        status: if available.is_empty() { Status::NotFound } else { Status::UpToDate },
        installed: None,
        latest: None,
        available,
        install_commands: Vec::new(),
    }
}

fn lookup_explicit(package: &str, sources: &[Box<dyn Source>], use_cache: bool) -> PackageResult {
    for source in sources {
        if let Some(version) = query_source(source, package, use_cache) {
            let info = VersionInfo {
                version,
                source: source.name().to_string(),
                local: source.is_local(),
            };
            return PackageResult {
                package: package.to_string(),
                status: Status::UpToDate,
                installed: Some(info.clone()),
                latest: Some(info),
                available: Vec::new(),
                install_commands: Vec::new(),
            };
        }
    }
    
    PackageResult {
        package: package.to_string(),
        status: Status::NotFound,
        installed: None,
        latest: None,
        available: Vec::new(),
        install_commands: Vec::new(),
    }
}

fn lookup_default(package: &str, sources: &[Box<dyn Source>], use_cache: bool) -> PackageResult {
    // Find installed version from local sources (parallel, never cached)
    let installed = sources.par_iter()
        .filter(|s| s.is_local())
        .find_map_any(|s| {
            s.get_version(package).map(|v| (v, s.name(), s.ecosystem()))
        });

    // Find versions from registries (parallel, cached)
    let registry_versions: Vec<_> = sources.par_iter()
        .filter(|s| !s.is_local())
        .filter_map(|s| {
            query_source(s, package, use_cache).map(|v| VersionInfo {
                version: v,
                source: s.name().to_string(),
                local: false,
            })
        })
        .collect();

    if let Some((inst_version, inst_source, inst_ecosystem)) = installed {
        // Check for newer version in same ecosystem only
        let newer = registry_versions.iter()
            .filter(|rv| sources.iter()
                .find(|s| s.name() == rv.source)
                .is_some_and(|s| s.ecosystem() == inst_ecosystem))
            .filter(|rv| is_newer(&inst_version, &rv.version))
            .max_by(|a, b| {
                // Compare versions to find newest
                if is_newer(&a.version, &b.version) { std::cmp::Ordering::Less }
                else { std::cmp::Ordering::Greater }
            });

        let installed_info = VersionInfo {
            version: inst_version,
            source: inst_source.to_string(),
            local: true,
        };

        if let Some(latest) = newer {
            PackageResult {
                package: package.to_string(),
                status: Status::Outdated,
                installed: Some(installed_info),
                latest: Some(latest.clone()),
                available: Vec::new(),
                install_commands: Vec::new(),
            }
        } else {
            PackageResult {
                package: package.to_string(),
                status: Status::UpToDate,
                installed: Some(installed_info.clone()),
                latest: Some(installed_info),
                available: Vec::new(),
                install_commands: Vec::new(),
            }
        }
    } else if !registry_versions.is_empty() {
        let first = registry_versions[0].clone();
        PackageResult {
            package: package.to_string(),
            status: Status::NotInstalled,
            installed: None,
            latest: Some(first),
            available: registry_versions.clone(),
            install_commands: get_install_commands(package, &registry_versions),
        }
    } else {
        PackageResult {
            package: package.to_string(),
            status: Status::NotFound,
            installed: None,
            latest: None,
            available: Vec::new(),
            install_commands: Vec::new(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Install commands
// ─────────────────────────────────────────────────────────────────────────────

fn get_install_commands(package: &str, available: &[VersionInfo]) -> Vec<String> {
    use std::path::Path;
    
    let in_uv = Path::new("uv.lock").exists();
    let in_node = Path::new("package.json").exists();
    let in_cargo = Path::new("Cargo.toml").exists();
    let in_go = Path::new("go.mod").exists();

    available.iter()
        .filter_map(|v| match v.source.as_str() {
            "brew" => Some(format!("brew install {}", package)),
            "npm" => Some(if in_node { format!("npm install {}", package) } else { format!("npm install -g {}", package) }),
            "pip" => Some(if in_uv { format!("uv add {}", package) } else { format!("pip install {}", package) }),
            "cargo" => Some(if in_cargo { format!("cargo add {}", package) } else { format!("cargo install {}", package) }),
            "go" => Some(if in_go { format!("go get {}", package) } else { format!("go install {}", package) }),
            _ => None,
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Output formatting
// ─────────────────────────────────────────────────────────────────────────────

fn format_result(r: &PackageResult, show_name: bool) -> String {
    let prefix = if show_name { format!("{}: ", r.package) } else { String::new() };
    
    match r.status {
        Status::UpToDate => format!("{}{}  ✓", prefix, r.installed.as_ref().unwrap().version),
        Status::Outdated => format!("{}{} → {} available", prefix, 
            r.installed.as_ref().unwrap().version,
            r.latest.as_ref().unwrap().version),
        Status::NotInstalled => {
            let avail: Vec<_> = r.available.iter()
                .map(|a| format!("{} in {}", a.version, a.source))
                .collect();
            format!("{}not installed (available: {})", prefix, avail.join(", "))
        }
        Status::NotFound => format!("{}not found", prefix),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let config = Config::load();

    // Get packages: from args or by scanning project
    let (packages, source_override) = if cli.packages.is_empty() {
        match project::scan() {
            Some(p) => {
                if !cli.json && !cli.quiet {
                    eprintln!("Scanning {}...", p.file);
                }
                (p.packages, Some(p.source))
            }
            None => {
                eprintln!("No project file found. Usage: latest <package> [...]");
                std::process::exit(1);
            }
        }
    } else {
        (cli.packages.clone(), None)
    };

    // Build source list
    let source_name = cli.source.as_deref().or(source_override);
    let sources: Vec<Box<dyn Source>> = if let Some(name) = source_name {
        match SourceType::from_name(name) {
            Some(st) => vec![st.create()],
            None => {
                eprintln!("Unknown source: {}", name);
                std::process::exit(1);
            }
        }
    } else {
        config.precedence.iter().map(|st| st.create()).collect()
    };

    // Determine lookup mode
    let mode = if cli.all {
        LookupMode::All
    } else if source_name.is_some() {
        LookupMode::Explicit
    } else {
        LookupMode::Default
    };

    // Lookup all packages (in parallel)
    let use_cache = !cli.no_cache;
    let results: Vec<_> = packages.par_iter()
        .map(|pkg| lookup(pkg, &sources, mode, use_cache))
        .collect();

    // Output
    if cli.json {
        let out = if results.len() == 1 {
            serde_json::to_string_pretty(&results[0]).unwrap()
        } else {
            serde_json::to_string_pretty(&results).unwrap()
        };
        println!("{}", out);
    } else if cli.quiet {
        for r in &results {
            let version = r.installed.as_ref().or(r.latest.as_ref());
            match version {
                Some(v) if results.len() > 1 => println!("{}: {}", r.package, v.version),
                Some(v) => println!("{}", v.version),
                None => eprintln!("not found: {}", r.package),
            }
        }
    } else if cli.all {
        for r in &results {
            if results.len() > 1 { println!("{}:", r.package); }
            if r.available.is_empty() {
                let line = if results.len() > 1 { "  not found" } else { "not found" };
                eprintln!("{}", line);
            } else {
                for v in &r.available {
                    let mark = if v.local { " (installed)" } else { "" };
                    let line = format!("{}: {}{}", v.source, v.version, mark);
                    if results.len() > 1 { println!("  {}", line); } else { println!("{}", line); }
                }
            }
        }
    } else {
        let multi = results.len() > 1;
        for r in &results {
            let line = format_result(r, multi);
            if matches!(r.status, Status::NotFound | Status::NotInstalled) {
                eprintln!("{}", line);
                for cmd in &r.install_commands {
                    eprintln!("  {}", cmd);
                }
            } else {
                println!("{}", line);
            }
        }
    }

    // Exit code
    let code = if results.iter().any(|r| matches!(r.status, Status::NotFound | Status::NotInstalled)) {
        1
    } else if results.iter().any(|r| r.status == Status::Outdated) {
        2
    } else {
        0
    };
    std::process::exit(code);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sources::Ecosystem;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("1.0.0", "1.0.1"));
        assert!(is_newer("1.0.0", "2.0.0"));
        assert!(is_newer("1.9.0", "1.10.0"));
        assert!(!is_newer("1.0.1", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    struct MockSource {
        name: &'static str,
        packages: Vec<(&'static str, &'static str)>,
        local: bool,
        ecosystem: Ecosystem,
    }

    impl Source for MockSource {
        fn name(&self) -> &'static str { self.name }
        fn is_local(&self) -> bool { self.local }
        fn ecosystem(&self) -> Ecosystem { self.ecosystem }
        fn get_version(&self, pkg: &str) -> Option<String> {
            self.packages.iter()
                .find(|(n, _)| *n == pkg)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn test_lookup_up_to_date() {
        let sources: Vec<Box<dyn Source>> = vec![
            Box::new(MockSource { name: "path", packages: vec![("node", "25.0.0")], local: true, ecosystem: Ecosystem::System }),
            Box::new(MockSource { name: "brew", packages: vec![("node", "25.0.0")], local: false, ecosystem: Ecosystem::System }),
        ];
        let r = lookup("node", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::UpToDate);
    }

    #[test]
    fn test_lookup_outdated_same_ecosystem() {
        let sources: Vec<Box<dyn Source>> = vec![
            Box::new(MockSource { name: "path", packages: vec![("node", "24.0.0")], local: true, ecosystem: Ecosystem::System }),
            Box::new(MockSource { name: "brew", packages: vec![("node", "25.0.0")], local: false, ecosystem: Ecosystem::System }),
        ];
        let r = lookup("node", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::Outdated);
    }

    #[test]
    fn test_lookup_not_outdated_different_ecosystem() {
        let sources: Vec<Box<dyn Source>> = vec![
            Box::new(MockSource { name: "path", packages: vec![("mcs", "0.7.0")], local: true, ecosystem: Ecosystem::System }),
            Box::new(MockSource { name: "npm", packages: vec![("mcs", "2.0.0")], local: false, ecosystem: Ecosystem::Npm }),
        ];
        let r = lookup("mcs", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::UpToDate); // Different ecosystem, not compared
    }

    #[test]
    fn test_lookup_not_installed() {
        let sources: Vec<Box<dyn Source>> = vec![
            Box::new(MockSource { name: "path", packages: vec![], local: true, ecosystem: Ecosystem::System }),
            Box::new(MockSource { name: "npm", packages: vec![("express", "5.0.0")], local: false, ecosystem: Ecosystem::Npm }),
        ];
        let r = lookup("express", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::NotInstalled);
        assert_eq!(r.available.len(), 1);
    }

    #[test]
    fn test_lookup_not_found() {
        let sources: Vec<Box<dyn Source>> = vec![
            Box::new(MockSource { name: "path", packages: vec![], local: true, ecosystem: Ecosystem::System }),
        ];
        let r = lookup("nonexistent", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::NotFound);
    }

    #[test]
    fn test_lookup_all_mode() {
        let sources: Vec<Box<dyn Source>> = vec![
            Box::new(MockSource { name: "path", packages: vec![("node", "25.0.0")], local: true, ecosystem: Ecosystem::System }),
            Box::new(MockSource { name: "npm", packages: vec![("node", "24.0.0")], local: false, ecosystem: Ecosystem::Npm }),
        ];
        let r = lookup("node", &sources, LookupMode::All, false);
        assert_eq!(r.available.len(), 2);
    }
}
