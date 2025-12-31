# latest - Technical Specification

`latest` is a CLI tool that finds the latest version of commands, packages, and libraries across multiple package managers and registries.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Concepts](#core-concepts)
3. [Sources](#sources)
4. [Project Scanning](#project-scanning)
5. [Version Comparison](#version-comparison)
6. [Output Formats](#output-formats)
7. [Configuration](#configuration)
8. [Extension Guide](#extension-guide)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                            CLI                                   │
│  (clap parser, args handling, output formatting)                │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Lookup Engine                            │
│  - Determines lookup mode (all/explicit/default)                │
│  - Coordinates source queries                                   │
│  - Compares versions within ecosystems                          │
│  - Generates install commands                                   │
└─────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┼───────────────┐
                ▼               ▼               ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Project Scanner │  │     Sources     │  │     Config      │
│  (Cargo.toml,   │  │  (path, brew,   │  │  (precedence,   │
│   package.json, │  │   npm, pip,     │  │   settings)     │
│   go.mod, etc.) │  │   cargo, go,    │  │                 │
│                 │  │   uv)           │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

### File Structure

```
src/
├── main.rs          # CLI, lookup engine, output formatting
├── config.rs        # Configuration loading (~/.config/latest/config.toml)
├── project.rs       # Project file scanning
└── sources/
    ├── mod.rs       # Source trait, Ecosystem enum, SourceType enum
    ├── path.rs      # $PATH binary lookup
    ├── brew.rs      # Homebrew
    ├── npm.rs       # npm registry
    ├── pip.rs       # PyPI
    ├── cargo.rs     # crates.io
    ├── go.rs        # Go modules
    └── uv.rs        # uv project-local Python packages
```

---

## Core Concepts

### Sources

A **Source** is anything that can provide version information for a package. Sources implement the `Source` trait:

```rust
pub trait Source {
    fn name(&self) -> &'static str;
    fn get_version(&self, package: &str) -> Option<String>;
    fn is_local(&self) -> bool { false }
    fn ecosystem(&self) -> Ecosystem;
}
```

- **`name()`** - Identifier used in output and `-s` flag (e.g., "npm", "cargo")
- **`get_version()`** - Returns the version string if the package exists
- **`is_local()`** - True if this source checks locally installed packages
- **`ecosystem()`** - Which ecosystem this source belongs to

### Ecosystems

Sources are grouped into **ecosystems**. Version comparisons only happen within the same ecosystem to avoid false "outdated" warnings when package names collide across registries.

```rust
pub enum Ecosystem {
    System,  // path, brew
    Python,  // uv, pip
    Npm,     // npm
    Cargo,   // cargo
    Go,      // go
}
```

Example: A binary `mcs` installed via PATH (v0.7.12) won't be compared against npm's `mcs` package (v2.2.1) because they're in different ecosystems.

### Local vs Registry Sources

| Type | Sources | Behavior |
|------|---------|----------|
| **Local** | path, uv | Check what's installed on the system |
| **Registry** | brew, npm, pip, cargo, go | Query remote registries for latest versions |

The default lookup mode:
1. First checks local sources for installed version
2. Then checks registry sources in the same ecosystem for updates
3. If not installed, shows what's available in registries

### Lookup Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| **Default** | No flags | Local-first, ecosystem-aware comparison |
| **Explicit** | `-s <source>` | Query only that source, return first match |
| **All** | `--all` | Query all sources, show everything found |

---

## Sources

### path (Local, System)

Checks if a command exists in `$PATH` and extracts version by trying common flags.

**Detection:**
```bash
which <package>
```

**Version extraction** (tries in order):
```bash
<package> --version
<package> -version
<package> version
<package> -V
<package> -v
```

Uses regex to extract semver-like patterns from output.

### brew (Registry, System)

Queries Homebrew for formula information.

**Query:**
```bash
brew info <package> --json=v2
```

**Parsing:** Extracts `versions.stable` from JSON response.

### npm (Registry, Npm)

Queries the npm registry.

**Query:**
```bash
npm view <package> version
```

Returns the version string directly.

### pip (Registry, Python)

Queries PyPI, with fallback to local install check.

**Local check:**
```bash
pip show <package>  # Parses "Version:" line
```

**Registry check:**
```bash
pip index versions <package>  # Extracts first version
```

Note: Tries both `pip` and `pip3` commands.

### cargo (Registry, Cargo)

Queries crates.io.

**Query:**
```bash
cargo search <package> --limit 1
```

**Parsing:** Matches exact package name in output format `name = "version"`.

### go (Registry, Go)

Queries the Go module proxy.

**Query:**
```bash
go list -m -versions <package>
```

**Parsing:** Takes the last (latest) version from the space-separated list.

### uv (Local, Python)

Checks uv project-local Python packages.

**Project detection:**
- `uv.lock` exists, OR
- `pyproject.toml` + `.venv` directory exist

**Version lookup:**
1. Parse `uv.lock` directly (fast, no subprocess)
2. Fallback: `uv pip show <package>`

---

## Project Scanning

When `latest` is run without arguments, it scans for project files.

### Supported Project Files

| File | Source | Language |
|------|--------|----------|
| `Cargo.toml` | cargo | Rust |
| `package.json` | npm | JavaScript/TypeScript |
| `uv.lock` | pip | Python (uv) |
| `pyproject.toml` | pip | Python (PEP 621) |
| `go.mod` | go | Go |

### Scan Order

Files are checked in this order (first match wins):
1. `Cargo.toml`
2. `package.json`
3. `uv.lock`
4. `pyproject.toml`
5. `go.mod`

### Parsing Details

**Cargo.toml:**
```toml
[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
criterion = "0.5"
```
Extracts keys from `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`.

**package.json:**
```json
{
  "dependencies": { "express": "^4.18.0" },
  "devDependencies": { "typescript": "^5.0.0" }
}
```
Extracts keys from `dependencies` and `devDependencies`.

**uv.lock:**
```toml
[[package]]
name = "flask"
version = "3.1.2"
```
Extracts `name` from each `[[package]]` block.

**pyproject.toml:**
```toml
[project]
dependencies = ["flask>=3.0", "requests"]
```
Parses PEP 508 dependency strings, extracting package names.

**go.mod:**
```go
require (
    github.com/gin-gonic/gin v1.9.1
    github.com/spf13/cobra v1.8.0
)
```
Extracts module paths from `require` blocks.

---

## Version Comparison

### Algorithm

```rust
fn is_newer(installed: &str, latest: &str) -> bool
```

1. Split both versions on non-digit characters
2. Parse each segment as u64
3. Compare segment by segment (missing segments treated as 0)
4. Return true if `latest` is greater

**Examples:**
- `1.0.0` vs `1.0.1` → true (patch bump)
- `1.9.0` vs `1.10.0` → true (handles numeric comparison correctly)
- `1.0.0` vs `1.0.0` → false (equal)
- `2.0.0` vs `1.9.9` → false (installed is newer)

### Ecosystem Filtering

Before comparing, versions are filtered by ecosystem:

```rust
if source.ecosystem() != installed_ecosystem {
    continue; // Skip comparison
}
```

This prevents false positives like:
- `go` (PATH binary v1.25.5) vs `go` (npm package v3.0.1)
- `mcs` (PATH binary v0.7.12) vs `mcs` (npm package v2.2.1)

---

## Output Formats

### Default (Human-Readable)

```
$ latest node
25.2.1  ✓

$ latest node flask
node: 25.2.1  ✓
flask: not installed (available: 3.1.2 in pip)
  pip install flask
```

**Status indicators:**
- `✓` - Up to date
- `→ X.Y.Z available` - Outdated
- `not installed (available: ...)` - Not installed
- `not found` - Not in any source

### JSON (`--json`)

```json
{
  "package": "node",
  "status": "up_to_date",
  "installed": {
    "version": "25.2.1",
    "source": "path",
    "local": true
  },
  "latest": {
    "version": "25.2.1",
    "source": "path",
    "local": true
  }
}
```

**Status values:** `up_to_date`, `outdated`, `not_installed`, `not_found`

**Fields:**
- `package` - Package name (always present)
- `status` - Status enum (always present)
- `installed` - Version info if installed (optional)
- `latest` - Latest/best version info (optional)
- `available` - Array of all found versions (for `not_installed`)
- `install_commands` - Suggested install commands (for `not_installed`)

### Quiet (`--quiet`)

```
$ latest -q node
25.2.1

$ latest -q node go
node: 25.2.1
go: 1.25.5
```

Just version numbers, no status indicators.

### All Sources (`--all`)

```
$ latest --all node
path: 25.2.1 (installed)
brew: 25.2.1
npm: 24.12.0
```

Shows version from every source that has the package.

---

## Configuration

### File Location

```
~/.config/latest/config.toml
```

### Options

```toml
# Source precedence (first match wins for default lookups)
precedence = ["path", "brew", "npm", "uv", "pip", "go", "cargo"]
```

### Default Precedence

1. path (local binaries)
2. brew (Homebrew)
3. npm (Node packages)
4. uv (Python project-local)
5. pip (Python global)
6. go (Go modules)
7. cargo (Rust crates)

---

## Extension Guide

### Adding a New Source

1. **Create the source file** (`src/sources/newname.rs`):

```rust
use super::{Ecosystem, Source};
use std::process::Command;

pub struct NewSource;

impl Source for NewSource {
    fn name(&self) -> &'static str {
        "newname"
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::NewEcosystem // or existing one
    }

    fn is_local(&self) -> bool {
        false // true if checking installed packages
    }

    fn get_version(&self, package: &str) -> Option<String> {
        let output = Command::new("newtool")
            .args(["info", package])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        // Parse version from output
        parse_version(&String::from_utf8_lossy(&output.stdout))
    }
}

fn parse_version(output: &str) -> Option<String> {
    // Custom parsing logic
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_properties() {
        assert_eq!(NewSource.name(), "newname");
        assert_eq!(NewSource.ecosystem(), Ecosystem::NewEcosystem);
    }
}
```

2. **Register in `src/sources/mod.rs`**:

```rust
mod newname;
pub use newname::NewSource;

// If adding new ecosystem:
pub enum Ecosystem {
    // ...existing...
    NewEcosystem,
}

pub enum SourceType {
    // ...existing...
    NewName,
}

impl SourceType {
    pub fn create(&self) -> Box<dyn Source> {
        match self {
            // ...existing...
            SourceType::NewName => Box::new(NewSource),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            // ...existing...
            "newname" => Some(SourceType::NewName),
            _ => None,
        }
    }
}
```

3. **Add to default precedence** in `src/config.rs`:

```rust
fn default_precedence() -> Vec<SourceType> {
    vec![
        // ...existing...
        SourceType::NewName,
    ]
}
```

4. **Update CLI help** in `src/main.rs`:

```rust
/// Only check a specific source (path, brew, npm, pip, go, cargo, uv, newname)
#[arg(short, long)]
source: Option<String>,
```

### Adding a New Project File

1. **Add scanner function** in `src/project.rs`:

```rust
fn scan_newproject() -> Option<ProjectInfo> {
    let content = fs::read_to_string("newproject.config").ok()?;
    
    // Parse the file format
    let packages: Vec<String> = parse_dependencies(&content);
    
    if packages.is_empty() { return None; }
    
    Some(ProjectInfo {
        file: "newproject.config",
        source: "newname",  // Which source to use
        packages,
    })
}
```

2. **Add to scan chain** in `src/project.rs`:

```rust
pub fn scan() -> Option<ProjectInfo> {
    scan_cargo()
        .or_else(scan_npm)
        // ...existing...
        .or_else(scan_newproject)  // Add here
}
```

3. **Add install command** in `src/main.rs`:

```rust
fn get_install_commands(package: &str, available: &[VersionInfo]) -> Vec<String> {
    let in_newproject = Path::new("newproject.config").exists();
    
    available.iter()
        .filter_map(|v| match v.source.as_str() {
            // ...existing...
            "newname" => Some(if in_newproject {
                format!("newtool add {}", package)
            } else {
                format!("newtool install {}", package)
            }),
            _ => None,
        })
        .collect()
}
```

### Adding New Output Formats

The output logic is in `main()`. To add a new format:

1. **Add CLI flag**:

```rust
#[arg(long)]
csv: bool,
```

2. **Add output branch**:

```rust
if cli.csv {
    println!("package,status,installed,latest");
    for r in &results {
        println!("{},{:?},{},{}",
            r.package,
            r.status,
            r.installed.as_ref().map(|v| &v.version).unwrap_or(&"".to_string()),
            r.latest.as_ref().map(|v| &v.version).unwrap_or(&"".to_string()),
        );
    }
}
```

### Testing

**Unit tests** go in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v1.2.3"), Some("1.2.3".to_string()));
    }
}
```

**Integration tests** go in `tests/integration_tests.rs`:

```rust
#[test]
fn test_new_source() {
    let output = latest_cmd()
        .args(["--source", "newname", "somepackage"])
        .output()
        .expect("Failed to run");
    
    assert!(output.status.success());
}
```

Run tests:
```bash
cargo test                    # All tests
cargo test test_name          # Specific test
cargo test -- --nocapture     # Show println output
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All packages up to date |
| 1 | Package not found or not installed |
| 2 | Package outdated |

---

## Performance Considerations

- **Source queries are sequential** - Each source is queried one at a time
- **Early exit in explicit mode** - Returns on first match
- **uv.lock parsing is fast** - No subprocess, direct file read
- **brew JSON parsing is simple** - String search, not full JSON parse

### Potential Optimizations

1. **Parallel source queries** - Use `rayon` or async
2. **Caching** - Cache registry responses with TTL
3. **Lazy regex compilation** - Currently compiled per-call in `extract_version`

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `serde` | Serialization (JSON output, config) |
| `serde_json` | JSON serialization |
| `toml` | TOML parsing (Cargo.toml, config, uv.lock) |
| `regex` | Version extraction from command output |
| `dirs` | Cross-platform config directory |
