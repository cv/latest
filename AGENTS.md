# Agent Instructions

## Project Overview

`latest` is a Rust CLI tool that finds the latest version of commands, packages, and libraries across multiple package managers. See [README.md](README.md) for usage. Technical details are in bd issues (`bd show <id>`).

## Development

```bash
cargo build           # Build
cargo test            # Run all tests (unit + integration)
cargo run -- <args>   # Run directly
cargo bench           # Run benchmarks (criterion)
cargo fmt             # Format code
cargo clippy          # Check lints
```

**Before committing**: Always run `cargo test`, `cargo clippy`, and `cargo build` to ensure nothing is broken.

### Property-Based Testing (proptest)

```bash
cargo test proptests                     # 256 random cases per test
PROPTEST_CASES=10000 cargo test proptests # More thorough
```

## Issue Tracking (bd)

This project uses **bd** for issue tracking.

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd create "title" -d "description" -p P2 -l label1,label2  # Create issue
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd list               # List all open issues
bd sync               # Sync issues to JSONL
```

**Priority scale**: P0 (critical) → P4 (lowest). Use P2 for medium, P3 for low.

## Git Workflow

- Work on `main` branch
- Make small, focused commits with clear messages
- Run tests before committing

## Landing the Plane (Session Completion)

When ending a session, complete ALL steps:

1. **Create issues** for any remaining/follow-up work
2. **Run quality gates** (if code changed):
   ```bash
   cargo test
   cargo build
   ```
3. **Update issues** - Close completed work, update in-progress items
4. **Commit and push**:
   ```bash
   git add -A
   git commit -m "descriptive message"
   git pull --rebase  # if remote exists
   bd sync
   git push           # if remote exists
   ```
5. **Verify** - `git status` shows clean working tree

**Critical**: Work is NOT complete until everything is committed. If a remote exists, push is mandatory.

## Delegating to Subagents

For well-defined, independent tasks, use subagents. This enables parallel execution and keeps the main agent focused on coordination.

### When to Use Subagents

- **Batch implementation**: Multiple independent issues/features
- **Code reviews**: Security audits, architecture review
- **Repetitive tasks**: Adding similar features (e.g., new package sources)

### Subagent Task Template

Tasks must be **completely self-contained**. Include:

```
## Project Location
/Users/carlosvillela/src/latest

## Problem
[Clear description of what needs to be done]

## Requirements
1. **TDD**: Write tests FIRST, then implement
2. **DRY**: Don't repeat yourself  
3. **DTSTTCPW**: Do the simplest thing that could possibly work
4. **Keep the linter happy**: Run `cargo clippy` and fix warnings

## Implementation Steps
1. [Specific steps]
2. [Including files to read/modify]
3. [Expected approach]

## Verification
- All existing tests must pass
- New tests for the feature must pass
- No clippy warnings
- Build succeeds (`cargo build --release`)
```

### Parallel Subagent Pattern

When implementing multiple independent issues:

```
1. Launch subagents in parallel (one per issue)
2. Wait for all to complete
3. Review changes: `git diff --stat HEAD`
4. Run quality gates: `cargo test && cargo clippy`
5. Close issues: `bd close <id>` for each
6. Commit all together with descriptive message
7. Push
```

### Example: Security Review → Issues → Implementation

This workflow worked well for the security hardening work:

1. **Review**: Subagent with high thinking does thorough security audit
2. **File issues**: Create bd issues for each finding with proper priority
3. **Implement**: Launch parallel subagents, one per issue
4. **Verify**: Review diffs, run tests, check clippy
5. **Ship**: Close issues, commit, push, release

## Releasing

```bash
# 1. Bump version
# Edit Cargo.toml: version = "X.Y.Z"

# 2. Build to update Cargo.lock
cargo build --release

# 3. Commit and tag
git add -A
git commit -m "chore: bump version to X.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z - Release description"

# 4. Push (triggers release workflow)
git push && git push --tags

# 5. Monitor release
gh run list --limit 3
gh release view vX.Y.Z
```

## Code Structure

```
src/
├── lib.rs           # Library crate (exposed for benchmarks/fuzz)
├── main.rs          # CLI, lookup engine, output formatting
├── cache.rs         # Response caching (~/.cache/latest/)
├── config.rs        # Configuration (~/.config/latest/config.toml)
├── project.rs       # Project file scanning
└── sources/
    ├── mod.rs       # Source trait, Ecosystem enum, JsonApiSource, define_sources! macro
    ├── path.rs      # $PATH binary lookup (local)
    ├── brew.rs      # Homebrew (macOS)
    ├── apt.rs       # APT packages (Debian/Ubuntu)
    ├── pip.rs       # pip show (local Python packages)
    ├── uv.rs        # uv project-local Python packages (local)
    ├── conda.rs     # Conda packages
    ├── composer.rs  # Packagist (PHP)
    ├── maven.rs     # Maven Central (JVM)
    ├── docker.rs    # Docker Hub
    ├── nuget.rs     # NuGet (.NET)
    └── swift.rs     # Swift Package Index
tests/
└── integration_tests.rs  # CLI integration tests
benches/
└── benchmarks.rs    # Criterion benchmarks
.github/workflows/
├── release.yml      # cargo-dist release automation
└── security.yml     # Daily cargo-audit vulnerability scanning
```

**Note**: npm, cargo, go, gem, hex, pub use `JsonApiSource` in mod.rs (no separate file).

## Adding New Sources

Most new sources can use `JsonApiSource` in `src/sources/mod.rs`:

```rust
static NEWSOURCE: JsonApiSource = JsonApiSource {
    name: "newsource",
    ecosystem: Ecosystem::NewEcosystem,  // Add to Ecosystem enum if needed
    url_template: "https://registry.example.com/packages/{}",
    version_path: "version",  // JSON path to version field (dot-separated)
};
```

Then register in the `define_sources!` macro in `src/sources/mod.rs`.

For complex sources needing custom logic, create `src/sources/newname.rs` implementing the `Source` trait.

**Checklist**:
1. Add source (JsonApiSource in mod.rs, or custom impl in new file)
2. Register in `define_sources!` macro in `src/sources/mod.rs`
3. Add tests (unit + integration)
4. Update SECURITY.md if source sends data to external services
