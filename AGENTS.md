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

### Fuzzing (requires nightly Rust)

```bash
rustup install nightly
cargo +nightly fuzz run fuzz_version_parsing
cargo +nightly fuzz run fuzz_package_arg
cargo +nightly fuzz run fuzz_config_parsing
```

## Issue Tracking (bd)

This project uses **bd** for issue tracking.

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd create             # Create a new issue
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd list               # List all open issues
```

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

## Code Structure

```
src/
├── lib.rs           # Library crate (exposed for benchmarks/fuzz)
├── main.rs          # CLI, lookup engine, output formatting
├── cache.rs         # Response caching (~/.cache/latest/)
├── config.rs        # Configuration (~/.config/latest/config.toml)
├── project.rs       # Project file scanning
└── sources/
    ├── mod.rs       # Source trait, Ecosystem enum
    ├── path.rs      # $PATH binary lookup
    ├── brew.rs      # Homebrew
    ├── apt.rs       # APT packages (Debian/Ubuntu)
    ├── pip.rs       # PyPI
    └── uv.rs        # uv project-local Python packages
benches/
└── benchmarks.rs    # Criterion benchmarks
fuzz/
├── Cargo.toml       # Fuzz target configuration
└── fuzz_targets/    # libfuzzer targets
```

## Adding New Sources

1. Create `src/sources/newname.rs` implementing the `Source` trait
2. Register in `src/sources/mod.rs` (add to `SourceType` enum and `from_name`)
3. Add to default precedence in `src/config.rs`
4. Add tests

See `bd show latest-rl5` for Source trait details.
