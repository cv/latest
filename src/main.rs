mod cache;
mod config;
mod project;
mod sources;

use clap::Parser;
use config::Config;
use rayon::prelude::*;
use sources::{source_by_name, Source};

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

impl VersionInfo {
    fn new(version: String, source: &dyn Source) -> Self {
        Self {
            version,
            source: source.name().to_string(),
            local: source.is_local(),
        }
    }
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

impl PackageResult {
    fn not_found(package: &str) -> Self {
        Self {
            package: package.to_string(),
            status: Status::NotFound,
            installed: None,
            latest: None,
            available: Vec::new(),
            install_commands: Vec::new(),
        }
    }

    fn up_to_date(package: &str, info: VersionInfo) -> Self {
        Self {
            package: package.to_string(),
            status: Status::UpToDate,
            installed: Some(info.clone()),
            latest: Some(info),
            available: Vec::new(),
            install_commands: Vec::new(),
        }
    }

    fn outdated(package: &str, installed: VersionInfo, latest: VersionInfo) -> Self {
        Self {
            package: package.to_string(),
            status: Status::Outdated,
            installed: Some(installed),
            latest: Some(latest),
            available: Vec::new(),
            install_commands: Vec::new(),
        }
    }

    fn not_installed(package: &str, available: Vec<VersionInfo>) -> Self {
        let install_commands = get_install_commands(package, &available);
        Self {
            package: package.to_string(),
            status: Status::NotInstalled,
            installed: None,
            latest: available.first().cloned(),
            available,
            install_commands,
        }
    }

    fn all_sources(package: &str, available: Vec<VersionInfo>) -> Self {
        Self {
            package: package.to_string(),
            status: if available.is_empty() { Status::NotFound } else { Status::UpToDate },
            installed: None,
            latest: None,
            available,
            install_commands: Vec::new(),
        }
    }
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
    (0..a.len().max(b.len())).any(|i| {
        let (x, y) = (*a.get(i).unwrap_or(&0), *b.get(i).unwrap_or(&0));
        x < y && (0..i).all(|j| a.get(j) == b.get(j))
    })
}

/// Query a source with optional caching (only for non-local sources)
fn query_source(source: &dyn Source, package: &str, use_cache: bool) -> Option<String> {
    if source.is_local() {
        return source.get_version(package);
    }
    if use_cache {
        if let Some(cached) = cache::get(source.name(), package) {
            return Some(cached);
        }
    }
    let version = source.get_version(package)?;
    if use_cache {
        cache::set(source.name(), package, &version);
    }
    Some(version)
}

#[derive(Clone, Copy)]
enum LookupMode {
    All,
    Explicit,
    Default,
}

fn lookup(package: &str, sources: &[Box<dyn Source>], mode: LookupMode, use_cache: bool) -> PackageResult {
    match mode {
        LookupMode::All => {
            let available: Vec<_> = sources.par_iter()
                .filter_map(|s| query_source(s.as_ref(), package, use_cache).map(|v| VersionInfo::new(v, s.as_ref())))
                .collect();
            PackageResult::all_sources(package, available)
        }
        LookupMode::Explicit => {
            sources.par_iter()
                .find_map_any(|s| query_source(s.as_ref(), package, use_cache).map(|v| VersionInfo::new(v, s.as_ref())))
                .map(|info| PackageResult::up_to_date(package, info))
                .unwrap_or_else(|| PackageResult::not_found(package))
        }
        LookupMode::Default => lookup_default(package, sources, use_cache),
    }
}

fn lookup_default(package: &str, sources: &[Box<dyn Source>], use_cache: bool) -> PackageResult {
    // Find installed version from local sources
    let installed = sources.par_iter()
        .filter(|s| s.is_local())
        .find_map_any(|s| s.get_version(package).map(|v| (v, s.as_ref())));

    // Find versions from registries
    let registry_versions: Vec<_> = sources.par_iter()
        .filter(|s| !s.is_local())
        .filter_map(|s| query_source(s.as_ref(), package, use_cache).map(|v| (v, s.as_ref())))
        .collect();

    match installed {
        Some((inst_version, inst_source)) => {
            let inst_ecosystem = inst_source.ecosystem();
            let newer = registry_versions.iter()
                .filter(|(_, s)| s.ecosystem() == inst_ecosystem)
                .filter(|(v, _)| is_newer(&inst_version, v))
                .max_by(|(a, _), (b, _)| if is_newer(a, b) { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater });

            let installed_info = VersionInfo::new(inst_version, inst_source);
            match newer {
                Some((v, s)) => PackageResult::outdated(package, installed_info, VersionInfo::new(v.clone(), *s)),
                None => PackageResult::up_to_date(package, installed_info),
            }
        }
        None if !registry_versions.is_empty() => {
            let available: Vec<_> = registry_versions.into_iter()
                .map(|(v, s)| VersionInfo::new(v, s))
                .collect();
            PackageResult::not_installed(package, available)
        }
        None => PackageResult::not_found(package),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Install commands
// ─────────────────────────────────────────────────────────────────────────────

fn get_install_commands(package: &str, available: &[VersionInfo]) -> Vec<String> {
    use std::path::Path;
    let context = (
        Path::new("uv.lock").exists(),
        Path::new("package.json").exists(),
        Path::new("Cargo.toml").exists(),
        Path::new("go.mod").exists(),
    );

    available.iter().filter_map(|v| {
        Some(match v.source.as_str() {
            "brew" => format!("brew install {}", package),
            "npm" => format!("npm install {}{}", if context.1 { "" } else { "-g " }, package),
            "pip" => format!("{} {}", if context.0 { "uv add" } else { "pip install" }, package),
            "cargo" => format!("cargo {} {}", if context.2 { "add" } else { "install" }, package),
            "go" => format!("go {} {}", if context.3 { "get" } else { "install" }, package),
            _ => return None,
        })
    }).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Output formatting
// ─────────────────────────────────────────────────────────────────────────────

fn format_result(r: &PackageResult, show_name: bool) -> String {
    let prefix = if show_name { format!("{}: ", r.package) } else { String::new() };
    match r.status {
        Status::UpToDate => format!("{}{}  ✓", prefix, r.installed.as_ref().unwrap().version),
        Status::Outdated => format!("{}{} → {} available", prefix, 
            r.installed.as_ref().unwrap().version, r.latest.as_ref().unwrap().version),
        Status::NotInstalled => {
            let avail = r.available.iter().map(|a| format!("{} in {}", a.version, a.source)).collect::<Vec<_>>().join(", ");
            format!("{}not installed (available: {})", prefix, avail)
        }
        Status::NotFound => format!("{}not found", prefix),
    }
}

fn output_results(cli: &Cli, results: &[PackageResult]) {
    if cli.json {
        let out = if results.len() == 1 {
            serde_json::to_string_pretty(&results[0]).unwrap()
        } else {
            serde_json::to_string_pretty(results).unwrap()
        };
        println!("{}", out);
    } else if cli.quiet {
        for r in results {
            match r.installed.as_ref().or(r.latest.as_ref()) {
                Some(v) if results.len() > 1 => println!("{}: {}", r.package, v.version),
                Some(v) => println!("{}", v.version),
                None => eprintln!("not found: {}", r.package),
            }
        }
    } else if cli.all {
        for r in results {
            if results.len() > 1 { println!("{}:", r.package); }
            if r.available.is_empty() {
                eprintln!("{}", if results.len() > 1 { "  not found" } else { "not found" });
            } else {
                for v in &r.available {
                    let mark = if v.local { " (installed)" } else { "" };
                    let line = format!("{}: {}{}", v.source, v.version, mark);
                    println!("{}", if results.len() > 1 { format!("  {}", line) } else { line });
                }
            }
        }
    } else {
        let multi = results.len() > 1;
        for r in results {
            let line = format_result(r, multi);
            if matches!(r.status, Status::NotFound | Status::NotInstalled) {
                eprintln!("{}", line);
                for cmd in &r.install_commands { eprintln!("  {}", cmd); }
            } else {
                println!("{}", line);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let config = Config::load();

    let (packages, source_override) = if cli.packages.is_empty() {
        match project::scan() {
            Some(p) => {
                if !cli.json && !cli.quiet { eprintln!("Scanning {}...", p.file); }
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

    let source_name = cli.source.as_deref().or(source_override);
    let sources: Vec<Box<dyn Source>> = match source_name {
        Some(name) => match source_by_name(name) {
            Some(s) => vec![s],
            None => { eprintln!("Unknown source: {}", name); std::process::exit(1); }
        },
        None => config.precedence.iter().map(|st| st.create()).collect(),
    };

    let mode = match (cli.all, source_name.is_some()) {
        (true, _) => LookupMode::All,
        (_, true) => LookupMode::Explicit,
        _ => LookupMode::Default,
    };

    let use_cache = !cli.no_cache;
    let results: Vec<_> = packages.par_iter()
        .map(|pkg| lookup(pkg, &sources, mode, use_cache))
        .collect();

    output_results(&cli, &results);

    let code = if results.iter().any(|r| matches!(r.status, Status::NotFound | Status::NotInstalled)) { 1 }
        else if results.iter().any(|r| r.status == Status::Outdated) { 2 }
        else { 0 };
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
            self.packages.iter().find(|(n, _)| *n == pkg).map(|(_, v)| v.to_string())
        }
    }

    fn mock(name: &'static str, packages: Vec<(&'static str, &'static str)>, local: bool, ecosystem: Ecosystem) -> Box<dyn Source> {
        Box::new(MockSource { name, packages, local, ecosystem })
    }

    #[test]
    fn test_lookup_up_to_date() {
        let sources = vec![
            mock("path", vec![("node", "25.0.0")], true, Ecosystem::System),
            mock("brew", vec![("node", "25.0.0")], false, Ecosystem::System),
        ];
        assert_eq!(lookup("node", &sources, LookupMode::Default, false).status, Status::UpToDate);
    }

    #[test]
    fn test_lookup_outdated_same_ecosystem() {
        let sources = vec![
            mock("path", vec![("node", "24.0.0")], true, Ecosystem::System),
            mock("brew", vec![("node", "25.0.0")], false, Ecosystem::System),
        ];
        assert_eq!(lookup("node", &sources, LookupMode::Default, false).status, Status::Outdated);
    }

    #[test]
    fn test_lookup_not_outdated_different_ecosystem() {
        let sources = vec![
            mock("path", vec![("mcs", "0.7.0")], true, Ecosystem::System),
            mock("npm", vec![("mcs", "2.0.0")], false, Ecosystem::Npm),
        ];
        assert_eq!(lookup("mcs", &sources, LookupMode::Default, false).status, Status::UpToDate);
    }

    #[test]
    fn test_lookup_not_installed() {
        let sources = vec![
            mock("path", vec![], true, Ecosystem::System),
            mock("npm", vec![("express", "5.0.0")], false, Ecosystem::Npm),
        ];
        let r = lookup("express", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::NotInstalled);
        assert_eq!(r.available.len(), 1);
    }

    #[test]
    fn test_lookup_not_found() {
        let sources = vec![mock("path", vec![], true, Ecosystem::System)];
        assert_eq!(lookup("nonexistent", &sources, LookupMode::Default, false).status, Status::NotFound);
    }

    #[test]
    fn test_lookup_all_mode() {
        let sources = vec![
            mock("path", vec![("node", "25.0.0")], true, Ecosystem::System),
            mock("npm", vec![("node", "24.0.0")], false, Ecosystem::Npm),
        ];
        assert_eq!(lookup("node", &sources, LookupMode::All, false).available.len(), 2);
    }
}
