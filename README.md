# latest

Find the latest version of any command, package, or library. Scans project files automatically.

## Quick Start

```bash
# Install
cargo install --path .

# Scan current project
cd my-rust-project
latest
# Scanning Cargo.toml...
# tokio: 1.48.0  ✓
# serde: 1.0.228  ✓

# Check specific packages
latest node go rust
```

## Features

- **7 sources**: path, brew, npm, pip, cargo, go, uv
- **Project scanning**: Auto-detects Cargo.toml, package.json, uv.lock, pyproject.toml, go.mod
- **Ecosystem-aware**: Compares versions within the same ecosystem (won't flag npm's `go` as newer than Go the language)
- **Context-aware install hints**: Suggests `cargo add` in a Cargo project, `npm install` (not `-g`) in a Node project
- **Multiple output formats**: Human-readable, JSON, quiet mode

## Usage

```
latest                        # Scan project files in current directory
latest <package>              # Check specific package(s)
latest npm:express            # Query specific source with prefix
latest --all node             # Show all sources
latest -s cargo serde         # Query specific source (alternative syntax)
latest --json                 # JSON output for scripting
latest -q node                # Quiet: just version number
```

## Output

| Output | Meaning |
|--------|---------|
| `pip: 0.6.0 (installed)` | Up to date, shows source |
| `npm: 24.0.0 → 25.2.1 available` | Outdated |
| `not installed (available: ...)` | Not installed, with install hints |
| `not found` | Package doesn't exist in any source |
| `⚠ Also found in: brew, npm` | Package exists in multiple ecosystems |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All packages up to date |
| 1 | Package not found or not installed |
| 2 | Package outdated |

## Sources

| Source | Type | Ecosystem | Description |
|--------|------|-----------|-------------|
| path | local | System | Commands in $PATH |
| brew | registry | System | Homebrew packages |
| npm | registry | Npm | npm registry |
| pip | local+registry | Python | PyPI (checks local first) |
| cargo | registry | Cargo | crates.io |
| go | registry | Go | Go module proxy |
| uv | local | Python | uv project-local packages |

## Project Scanning

When run without arguments, `latest` scans for project files in this order:

| File | Source | Language |
|------|--------|----------|
| `Cargo.toml` | cargo | Rust |
| `package.json` | npm | Node.js |
| `uv.lock` | pip | Python (uv) |
| `pyproject.toml` | pip | Python |
| `go.mod` | go | Go |

## Configuration

Create `~/.config/latest/config.toml` to customize source precedence:

```toml
precedence = ["path", "brew", "npm", "uv", "pip", "go", "cargo"]
```

## Examples

```bash
# Check if project dependencies are current
latest

# Check a package (shows source and cross-ecosystem warnings)
latest latest
# pip: 0.6.0 (installed)
# ⚠ Also found in: brew, npm, cargo, gem

# Query a specific ecosystem with prefix syntax
latest npm:latest
# npm: 0.2.0

# JSON output for scripting
latest --json | jq '.[] | select(.status != "up_to_date")'

# Check all sources for a package
latest --all node
# path: 25.2.1 (installed)
# brew: 25.2.1
# npm: 24.12.0

# Quiet mode for scripts (version only, no source prefix)
latest -q -s npm express
# 5.2.1
```

## Development

```bash
cargo build           # Build
cargo test            # Run tests
cargo run -- node     # Run directly
```

## License

MIT
