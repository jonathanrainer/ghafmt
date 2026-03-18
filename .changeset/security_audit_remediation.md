---
default: patch
---

# Security audit remediation

Address findings from an internal security audit of the binary, CI/CD pipeline,
supply chain, and repository hygiene.

**Supply chain:**
- Pin `fyaml` and `fyaml-sys` patch dependencies to exact commit SHAs (`rev =`)
  instead of a mutable branch, preventing silent updates on `cargo update`
- Pin `similar` to `=2.7.0` for consistency with all other exact-pinned deps

**CI/CD:**
- Replace the unauthenticated `curl | bash` actionlint install with a pinned
  release download and SHA-256 checksum verification (version sourced from the
  release checksums file, so Renovate bumps only the version)
- Remove dead Windows packaging code from `_build_artifacts.yml` (Windows is not
  in the build matrix)
- Generate `.sha256` files alongside release tarballs and verify them in
  `action.yml` before extracting the downloaded binary
- Digest-pin the Dockerfile base images (`amd64-musl` and `arm64-musl`) using
  per-platform digest pinning via named multi-stage `FROM` aliases
- Pin the fallback `yq` download in `set-cargo-version` to a specific release
  with checksum verification (SHA-256 position derived from `checksums_hashes_order`)

**Binary hardening:**
- Disable symlink following in `--write` mode to prevent path-traversal writes
  via symlinks pointing outside the target directory
- Atomic file writes via `tempfile::NamedTempFile` + rename, preventing partial
  files on interrupted writes
- Cap stdin consumption at 10 MB, returning an error if the limit is exceeded
- Source code embedded in parse error diagnostics is now limited to ±5 lines
  around the error, preventing secrets elsewhere in the file from leaking to stderr
- Replace `.unwrap()` with `.unwrap_or_default()` in YAML sort closures to avoid
  panics on non-scalar keys

**Repository hygiene:**
- Add `SECURITY.md` with a private vulnerability disclosure path
- Add explanatory comments to suppressed advisory and `multiple-versions = "allow"`
  in `deny.toml`