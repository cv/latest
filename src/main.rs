use clap::Parser;
use latest::cache;
use latest::config::Config;
use latest::project;
use latest::sources::{self, Source, source_by_name};
use rayon::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Security: Output sanitization
// ─────────────────────────────────────────────────────────────────────────────

/// Sanitize output to prevent escape sequence injection from malicious sources.
/// Strips all control characters except newline, including ANSI escape sequences.
fn sanitize_output(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ANSI escape sequence: ESC [ ... final_byte
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter (final byte of CSI sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // ESC without [ is also stripped (it's a control char)
        } else if !c.is_control() || c == '\n' {
            result.push(c);
        }
    }
    result
}

#[derive(Parser)]
#[command(name = "latest")]
#[command(about = "Find the latest version of any command, package, or library")]
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    /// Packages to look up (if empty, scans project files)
    packages: Vec<String>,

    /// Only check a specific source (e.g., npm, cargo, brew, pip, go)
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

    /// Only use local sources (no network requests)
    #[arg(long)]
    offline: bool,
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
    fn new(version: &str, source: &dyn Source) -> Self {
        Self {
            version: sanitize_output(version),
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
    /// Other sources where the package was found (for clash warnings)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    also_found_in: Vec<String>,
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
            also_found_in: Vec::new(),
        }
    }

    fn up_to_date(package: &str, info: VersionInfo, also_found_in: Vec<String>) -> Self {
        Self {
            package: package.to_string(),
            status: Status::UpToDate,
            installed: Some(info.clone()),
            latest: Some(info),
            available: Vec::new(),
            install_commands: Vec::new(),
            also_found_in,
        }
    }

    fn outdated(
        package: &str,
        installed: VersionInfo,
        latest: VersionInfo,
        also_found_in: Vec<String>,
    ) -> Self {
        Self {
            package: package.to_string(),
            status: Status::Outdated,
            installed: Some(installed),
            latest: Some(latest),
            available: Vec::new(),
            install_commands: Vec::new(),
            also_found_in,
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
            also_found_in: Vec::new(),
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
            also_found_in: Vec::new(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────────────────────

use latest::{is_newer, parse_package_arg};

/// Query a source with optional caching (only for non-local sources)
#[allow(clippy::collapsible_if)] // Let chains require nightly rustfmt
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

fn lookup(
    package: &str,
    sources: &[Box<dyn Source>],
    mode: LookupMode,
    use_cache: bool,
) -> PackageResult {
    match mode {
        LookupMode::All => {
            let available: Vec<_> = sources
                .par_iter()
                .filter_map(|s| {
                    query_source(s.as_ref(), package, use_cache)
                        .map(|v| VersionInfo::new(&v, s.as_ref()))
                })
                .collect();
            PackageResult::all_sources(package, available)
        }
        LookupMode::Explicit => sources
            .par_iter()
            .find_map_any(|s| {
                query_source(s.as_ref(), package, use_cache)
                    .map(|v| VersionInfo::new(&v, s.as_ref()))
            })
            .map_or_else(
                || PackageResult::not_found(package),
                |info| PackageResult::up_to_date(package, info, Vec::new()),
            ),
        LookupMode::Default => lookup_default(package, sources, use_cache),
    }
}

fn lookup_default(package: &str, sources: &[Box<dyn Source>], use_cache: bool) -> PackageResult {
    // Find installed version from local sources
    let installed = sources
        .par_iter()
        .filter(|s| s.is_local())
        .find_map_any(|s| s.get_version(package).map(|v| (v, s.as_ref())));

    // Find versions from registries
    let registry_versions: Vec<_> = sources
        .par_iter()
        .filter(|s| !s.is_local())
        .filter_map(|s| query_source(s.as_ref(), package, use_cache).map(|v| (v, s.as_ref())))
        .collect();

    match installed {
        Some((inst_version, inst_source)) => {
            let inst_ecosystem = inst_source.ecosystem();
            let newer = registry_versions
                .iter()
                .filter(|(_, s)| s.ecosystem() == inst_ecosystem)
                .filter(|(v, _)| is_newer(&inst_version, v))
                .max_by(|(a, _), (b, _)| {
                    if is_newer(a, b) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                });

            // Collect other sources where the package was found (for clash warning)
            let also_found_in: Vec<String> = registry_versions
                .iter()
                .filter(|(_, s)| s.ecosystem() != inst_ecosystem)
                .map(|(_, s)| s.name().to_string())
                .collect();

            let installed_info = VersionInfo::new(&inst_version, inst_source);
            match newer {
                Some((v, s)) => PackageResult::outdated(
                    package,
                    installed_info,
                    VersionInfo::new(v, *s),
                    also_found_in,
                ),
                None => PackageResult::up_to_date(package, installed_info, also_found_in),
            }
        }
        None if !registry_versions.is_empty() => {
            let available: Vec<_> =
                registry_versions.into_iter().map(|(v, s)| VersionInfo::new(&v, s)).collect();
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

    available
        .iter()
        .filter_map(|v| {
            Some(match v.source.as_str() {
                "brew" => format!("brew install {package}"),
                "npm" => format!("npm install {}{}", if context.1 { "" } else { "-g " }, package),
                "pip" => {
                    format!("{} {}", if context.0 { "uv add" } else { "pip install" }, package)
                }
                "cargo" => {
                    format!("cargo {} {}", if context.2 { "add" } else { "install" }, package)
                }
                "go" => format!("go {} {}", if context.3 { "get" } else { "install" }, package),
                _ => return None,
            })
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Output formatting
// ─────────────────────────────────────────────────────────────────────────────

/// Format a package result for display.
/// Uses unwrap on installed/latest because status guarantees their presence.
#[allow(clippy::unwrap_used)]
fn format_result(r: &PackageResult, show_name: bool) -> String {
    let pkg_prefix = if show_name { format!("{}: ", r.package) } else { String::new() };
    match r.status {
        Status::UpToDate => {
            let info = r.installed.as_ref().unwrap();
            let installed_marker = if info.local { " (installed)" } else { "" };
            format!("{pkg_prefix}{}: {}{}", info.source, info.version, installed_marker)
        }
        Status::Outdated => {
            let installed = r.installed.as_ref().unwrap();
            let latest = &r.latest.as_ref().unwrap().version;
            let installed_marker = if installed.local { " (installed)" } else { "" };
            format!(
                "{pkg_prefix}{}: {}{} → {} available",
                installed.source, installed.version, installed_marker, latest
            )
        }
        Status::NotInstalled => {
            let avail = r
                .available
                .iter()
                .map(|a| format!("{} in {}", a.version, a.source))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{pkg_prefix}not installed (available: {avail})")
        }
        Status::NotFound => format!("{pkg_prefix}not found"),
    }
}

#[allow(clippy::unwrap_used)]
fn output_results(cli: &Cli, results: &[PackageResult]) {
    if cli.json {
        // JSON serialization of simple structs won't fail
        let out = if results.len() == 1 {
            serde_json::to_string_pretty(&results[0]).unwrap()
        } else {
            serde_json::to_string_pretty(results).unwrap()
        };
        println!("{out}");
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
            if results.len() > 1 {
                println!("{}:", r.package);
            }
            if r.available.is_empty() {
                eprintln!("{}", if results.len() > 1 { "  not found" } else { "not found" });
            } else {
                for v in &r.available {
                    let mark = if v.local { " (installed)" } else { "" };
                    let line = format!("{}: {}{}", v.source, v.version, mark);
                    println!("{}", if results.len() > 1 { format!("  {line}") } else { line });
                }
            }
        }
    } else {
        let multi = results.len() > 1;
        for r in results {
            let line = format_result(r, multi);
            if matches!(r.status, Status::NotFound | Status::NotInstalled) {
                eprintln!("{line}");
                for cmd in &r.install_commands {
                    eprintln!("  {cmd}");
                }
            } else {
                println!("{line}");
                // Show clash warning if package was found in other ecosystems
                if !r.also_found_in.is_empty() {
                    eprintln!("⚠ Also found in: {}", r.also_found_in.join(", "));
                }
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

    let (packages, source_override): (Vec<(Option<String>, String)>, Option<&str>) =
        if cli.packages.is_empty() {
            if let Some(p) = project::scan() {
                if !cli.json && !cli.quiet {
                    eprintln!("Scanning {}...", p.file);
                }
                (p.packages.into_iter().map(|s| (None, s)).collect(), Some(p.source))
            } else {
                eprintln!("No project file found. Usage: latest <package> [...]");
                std::process::exit(1);
            }
        } else {
            (cli.packages.iter().map(|s| parse_package_arg(s)).collect(), None)
        };

    // Global source override from --source flag or project detection
    let global_source = cli.source.as_deref().or(source_override);

    // Validate global source if specified via --source
    if let Some(name) = cli.source.as_deref()
        && source_by_name(name).is_none()
    {
        eprintln!("Unknown source: {name}");
        std::process::exit(1);
    }

    let use_cache = !cli.no_cache;

    let results: Vec<_> = packages
        .par_iter()
        .map(|(prefix_source, pkg)| {
            // Prefix source takes priority over global source
            let source_name = prefix_source.as_deref().or(global_source);

            let sources_to_use: Vec<Box<dyn Source>> = match source_name {
                Some(name) => source_by_name(name).map_or_else(Vec::new, |s| vec![s]),
                None => config.precedence.iter().map(sources::SourceType::create).collect(),
            };

            // Filter to local-only sources when offline mode is enabled
            let sources_to_use: Vec<Box<dyn Source>> = if cli.offline {
                sources_to_use.into_iter().filter(|s| s.is_local()).collect()
            } else {
                sources_to_use
            };

            if sources_to_use.is_empty() {
                // This happens if an unknown source was specified
                return PackageResult::not_found(pkg);
            }

            let mode = match (cli.all, source_name.is_some()) {
                (true, _) => LookupMode::All,
                (_, true) => LookupMode::Explicit,
                _ => LookupMode::Default,
            };

            lookup(pkg, &sources_to_use, mode, use_cache)
        })
        .collect();

    output_results(&cli, &results);

    let code =
        if results.iter().any(|r| matches!(r.status, Status::NotFound | Status::NotInstalled)) {
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
    use latest::sources::Ecosystem;

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
        fn name(&self) -> &'static str {
            self.name
        }
        fn is_local(&self) -> bool {
            self.local
        }
        fn ecosystem(&self) -> Ecosystem {
            self.ecosystem
        }
        fn get_version(&self, pkg: &str) -> Option<String> {
            self.packages.iter().find(|(n, _)| *n == pkg).map(|(_, v)| v.to_string())
        }
    }

    fn mock(
        name: &'static str,
        packages: Vec<(&'static str, &'static str)>,
        local: bool,
        ecosystem: Ecosystem,
    ) -> Box<dyn Source> {
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
        assert_eq!(
            lookup("nonexistent", &sources, LookupMode::Default, false).status,
            Status::NotFound
        );
    }

    #[test]
    fn test_lookup_all_mode() {
        let sources = vec![
            mock("path", vec![("node", "25.0.0")], true, Ecosystem::System),
            mock("npm", vec![("node", "24.0.0")], false, Ecosystem::Npm),
        ];
        assert_eq!(lookup("node", &sources, LookupMode::All, false).available.len(), 2);
    }

    #[test]
    fn test_parse_package_arg_with_prefix() {
        let (source, pkg) = parse_package_arg("npm:express");
        assert_eq!(source, Some("npm".to_string()));
        assert_eq!(pkg, "express");
    }

    #[test]
    fn test_parse_package_arg_without_prefix() {
        let (source, pkg) = parse_package_arg("express");
        assert_eq!(source, None);
        assert_eq!(pkg, "express");
    }

    #[test]
    fn test_parse_package_arg_unknown_prefix() {
        // Unknown prefix should not be treated as a source
        let (source, pkg) = parse_package_arg("unknown:express");
        assert_eq!(source, None);
        assert_eq!(pkg, "unknown:express");
    }

    #[test]
    fn test_also_found_in_different_ecosystem() {
        let sources = vec![
            mock("path", vec![("pkg", "1.0.0")], true, Ecosystem::System),
            mock("brew", vec![("pkg", "1.0.0")], false, Ecosystem::System),
            mock("npm", vec![("pkg", "2.0.0")], false, Ecosystem::Npm),
            mock("cargo", vec![("pkg", "3.0.0")], false, Ecosystem::Cargo),
        ];
        let r = lookup("pkg", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::UpToDate);
        // npm and cargo are different ecosystems, so they should be in also_found_in
        assert!(r.also_found_in.contains(&"npm".to_string()));
        assert!(r.also_found_in.contains(&"cargo".to_string()));
        // brew is same ecosystem as path, so it should not be in also_found_in
        assert!(!r.also_found_in.contains(&"brew".to_string()));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sanitization tests (TDD: tests written first)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_sanitize_normal_version_unchanged() {
        assert_eq!(sanitize_output("1.0.0"), "1.0.0");
        assert_eq!(sanitize_output("v2.3.4-beta.1"), "v2.3.4-beta.1");
        assert_eq!(sanitize_output("25.0.0"), "25.0.0");
    }

    #[test]
    fn test_sanitize_strips_ansi_escape_sequences() {
        // ANSI color codes
        assert_eq!(sanitize_output("\x1b[31m1.0.0\x1b[0m"), "1.0.0");
        // ANSI clear screen
        assert_eq!(sanitize_output("\x1b[2J1.0.0"), "1.0.0");
        // ANSI cursor movement
        assert_eq!(sanitize_output("\x1b[H1.0.0"), "1.0.0");
    }

    #[test]
    fn test_sanitize_strips_control_characters() {
        // Tab character
        assert_eq!(sanitize_output("1.0\t.0"), "1.0.0");
        // Null byte
        assert_eq!(sanitize_output("1.0\x00.0"), "1.0.0");
        // Form feed
        assert_eq!(sanitize_output("1.0\x0C.0"), "1.0.0");
    }

    #[test]
    fn test_sanitize_preserves_newline() {
        assert_eq!(sanitize_output("1.0.0\n"), "1.0.0\n");
        assert_eq!(sanitize_output("line1\nline2"), "line1\nline2");
    }

    #[test]
    fn test_sanitize_strips_carriage_return() {
        assert_eq!(sanitize_output("1.0.0\r"), "1.0.0");
        assert_eq!(sanitize_output("fake\rreal"), "fakereal");
    }

    #[test]
    fn test_sanitize_strips_bell_character() {
        assert_eq!(sanitize_output("1.0.0\x07"), "1.0.0");
        assert_eq!(sanitize_output("\x07\x07alert\x07"), "alert");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Offline mode tests (TDD: tests written first for issue latest-8y4)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_filter_sources_for_offline_mode() {
        let sources: Vec<Box<dyn Source>> = vec![
            mock("path", vec![("node", "25.0.0")], true, Ecosystem::System),
            mock("brew", vec![("node", "25.0.0")], false, Ecosystem::System),
            mock("npm", vec![("express", "5.0.0")], false, Ecosystem::Npm),
            mock("uv", vec![("requests", "2.0.0")], true, Ecosystem::Python),
        ];

        let local_only: Vec<_> = sources.into_iter().filter(|s| s.is_local()).collect();

        assert_eq!(local_only.len(), 2);
        assert!(local_only.iter().any(|s| s.name() == "path"));
        assert!(local_only.iter().any(|s| s.name() == "uv"));
        assert!(!local_only.iter().any(|s| s.name() == "brew"));
        assert!(!local_only.iter().any(|s| s.name() == "npm"));
    }

    #[test]
    fn test_offline_lookup_finds_local_package() {
        let sources: Vec<Box<dyn Source>> = vec![
            mock("path", vec![("node", "25.0.0")], true, Ecosystem::System),
        ];

        let r = lookup("node", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::UpToDate);
    }

    #[test]
    fn test_offline_lookup_not_found_when_only_network() {
        // No local sources have the package
        let sources: Vec<Box<dyn Source>> = vec![
            mock("path", vec![], true, Ecosystem::System),
        ];

        let r = lookup("express", &sources, LookupMode::Default, false);
        assert_eq!(r.status, Status::NotFound);
    }

    #[test]
    fn test_offline_mode_all_flag_only_shows_local() {
        let sources: Vec<Box<dyn Source>> = vec![
            mock("path", vec![("node", "25.0.0")], true, Ecosystem::System),
            mock("uv", vec![("node", "24.0.0")], true, Ecosystem::Python),
        ];

        let r = lookup("node", &sources, LookupMode::All, false);
        assert_eq!(r.available.len(), 2);
        assert!(r.available.iter().all(|v| v.local));
    }
}
