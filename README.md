# latest

Find the latest version of any command, package, or library. Scans project files automatically.

```bash
$ cd my-rust-project
$ latest
Scanning Cargo.toml...
tokio: 1.48.0  ✓
serde: 1.0.228  ✓
axum: 0.8.8  ✓

$ latest flask
not installed (available: 3.1.2 in pip)
  pip install flask
```

## Installation

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
./target/release/latest
```

## Usage

```
latest                        # Scan project files in current directory
latest <package>              # Check specific package(s)
latest --all node             # Show all sources
latest -s cargo serde         # Query specific source
latest --json                 # JSON output for scripting
latest -q node                # Quiet: just version number
```

### Project Scanning

When run without arguments, `latest` automatically detects and scans:

| File | Source | Example |
|------|--------|---------|
| `Cargo.toml` | cargo | Rust projects |
| `package.json` | npm | Node.js projects |
| `uv.lock` | pip | uv Python projects |
| `pyproject.toml` | pip | Python projects |
| `go.mod` | go | Go modules |

```bash
$ cd my-node-project
$ latest
Scanning package.json...
express: 5.2.1  ✓
typescript: 5.9.3  ✓
vitest: 4.0.16  ✓

$ cd my-python-project  
$ latest
Scanning uv.lock...
flask: 3.1.2  ✓
requests: 2.32.5  ✓
```

### Output

- `25.2.1  ✓` - Up to date
- `24.0.0 → 25.2.1 available` - Outdated
- `not installed (available: ...)` - Not installed, with install hints
- `not found` - Package doesn't exist

### Exit Codes

- `0` - All packages up to date
- `1` - Package not found or not installed
- `2` - Package outdated

### Sources

**Local sources** (checks what's installed):
- **path** - Commands in your $PATH
- **uv** - Python packages in uv projects (reads `uv.lock`)

**Registry sources** (checks what's available):
- **brew** - Homebrew packages
- **npm** - npm registry
- **pip** - PyPI packages
- **go** - Go modules
- **cargo** - Rust crates (crates.io)

### Context-Aware Install Hints

Install commands adapt to your project context:

```bash
# In a Node project (has package.json)
$ latest express
not installed (available: 5.2.1 in npm)
  npm install express          # Not -g!

# In a uv Python project (has uv.lock)
$ latest requests
not installed (available: 2.32.5 in pip)
  uv add requests              # Uses uv, not pip!

# In a Cargo project (has Cargo.toml)
$ latest serde
not installed (available: 1.0.228 in cargo)
  cargo add serde              # Not cargo install!

# In a Go module (has go.mod)
$ latest github.com/spf13/cobra
not installed (available: 1.10.2 in go)
  go get github.com/spf13/cobra  # Not go install!
```

## Configuration

Create `~/.config/latest/config.toml` to customize source precedence:

```toml
# Default precedence (first match wins)
precedence = ["path", "brew", "npm", "uv", "pip", "go", "cargo"]
```

## Examples

```bash
# Scan current project
$ latest
Scanning Cargo.toml...
tokio: 1.48.0  ✓
serde: 1.0.228  ✓

# Check specific packages
$ latest node go rust
node: 25.2.1  ✓
go: 1.25.5  ✓
rust: 1.92.0  ✓

# Check all sources for a package
$ latest --all node
path: 25.2.1 (installed)
brew: 25.2.1
npm: 24.12.0

# Query specific registry
$ latest -s cargo tokio serde axum
tokio: 1.48.0  ✓
serde: 1.0.228  ✓
axum: 0.8.8  ✓

# JSON output for scripting
$ latest --json
[
  {"package":"tokio","status":"up_to_date","installed":{"version":"1.48.0",...}},
  {"package":"serde","status":"up_to_date","installed":{"version":"1.0.228",...}}
]

# Quiet mode for scripts
$ latest -q -s npm express
5.2.1
```

## AI Agent Usage

```bash
# Scan project dependencies
$ latest --json | jq '.[] | select(.status != "up_to_date")'

# Check if all deps are current (exit code 0 = all good)
$ latest && echo "All up to date!"

# Get install commands for missing packages
$ latest --json flask | jq '.install_commands'
```

## Known Limitations

- Package names can collide across registries (e.g., npm's `go` vs Go the language)
- Use `-s <source>` to query a specific registry when names are ambiguous
- Project scanning checks latest available, not if your pinned version is outdated

## License

MIT
