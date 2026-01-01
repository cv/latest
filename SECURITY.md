# Security Model

This document describes the security characteristics of `latest` for users evaluating its use in different environments.

## Data Sent to External Services

When querying package versions, `latest` sends **package names** to public registries:

| Source | Registry | Data Sent |
|--------|----------|-----------|
| npm | registry.npmjs.org | Package name |
| cargo | crates.io | Package name |
| go | proxy.golang.org | Module path |
| gem | rubygems.org | Gem name |
| hex | hex.pm | Package name |
| pub | pub.dev | Package name |
| brew | Homebrew API (via `brew` CLI) | Formula name |
| apt | Configured apt sources (via `apt` CLI) | Package name |
| pip | PyPI (via `pip` CLI) | Package name |

Requests are made using `curl` with default headers. No authentication tokens or identifying information beyond standard HTTP headers are sent.

## Local Command Execution

The **path** source discovers installed versions by executing binaries:

1. Checks if the command exists in `$PATH` using `which`
2. Executes the binary with version flags: `--version`, `-version`, `version`, `-V`
3. Parses stdout/stderr for version numbers

**Implication**: Any binary in your `$PATH` may be executed when queried. Only query packages you trust.

### PATH Source Risks in Shared Environments

In shared or multi-user environments, the PATH source poses additional risks:

- **PATH manipulation**: Other users or processes may place malicious binaries earlier in your `$PATH`. When you query `latest node`, a trojan `node` binary could be executed.
- **Untrusted directories in PATH**: If `$PATH` includes world-writable directories (e.g., `/tmp`), attackers could plant binaries there.
- **Command injection via package names**: While `latest` validates that binaries exist via `which` before execution, user-supplied package names still become command arguments.

### Mitigating PATH Source Risks

To avoid executing local binaries entirely, use registry-only sources:

```bash
# Use specific registry sources instead of path
latest -s npm express       # Query npm registry only
latest -s cargo serde       # Query crates.io only
latest npm:express          # Prefix syntax also avoids path source

# For project scanning, path source is not used (project files specify ecosystems)
latest                      # Safe: uses cargo/npm/pip based on project files
```

**For CI/CD pipelines**: Consider whether you need local version detection. If you only need to check registry versions, use explicit source flags to avoid the path source entirely.

## Fingerprinting Risk

`latest` can enumerate installed software, which may be a concern in some environments:

- `latest --all <package>` reveals which package managers have a package installed
- Project scanning (`latest` with no arguments) enumerates dependencies from project files
- The path source can detect any executable in `$PATH`

**Recommendation for shared/CI environments**: Be aware that output may reveal your software inventory. Consider using `--json` with filtering if you need to limit exposed information.

## Cache Storage

Cached data is stored in `~/.cache/latest/`. This cache contains:

- Registry responses (package versions) with a 1-hour TTL
- Files are named `{source}-{package}.json`

Cache files are readable only by the current user (standard umask). No credentials are cached.

## Recommended Practices

| Environment | Guidance |
|-------------|----------|
| **Local development** | Safe for normal use |
| **CI/CD pipelines** | Avoid PATH source if untrusted code may modify `$PATH`; use `-s <source>` to query specific registries |
| **Air-gapped systems** | Use `--source path` to avoid network calls; only local sources will work |
| **Shared servers** | Verify `$PATH` integrity before using path source; other users may see queries in process lists |

## Reporting Security Issues

To report a security vulnerability, please open an issue or contact the maintainers directly.
