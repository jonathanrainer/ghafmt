## 0.1.1 (2026-03-24)

### Fixes

#### Fix step ID references in bare `if:` conditions not being updated

When a step ID was renamed (e.g. `cache-thing` → `cache_thing`), references
to that ID in `if:` conditions written without `${{ }}` delimiters were left
unchanged, producing a broken workflow.

GitHub Actions allows `if:` fields to omit the expression wrapper, so
`if: steps.cache-thing.outputs.cache-hit != 'true'` is valid. The reference
updater now processes `/if` paths regardless of whether they contain `${{`. (453bd89)

### Project Infrastructure

#### Regenerate Cargo.lock during prepare-release

Added a `cargo update -p ghafmt` step to the `prepare-release` workflow so
that `Cargo.lock` reflects the new version before the release commit is made. (a05b484)

#### Harden workflows ahead of making repository public

Add explicit `permissions: contents: read` to all read-only jobs in
`code_checks.yml`, `workflow_checks.yml`, and the `setup` job in
`nightly_build.yml`, so that a default-permissions change cannot silently
grant write access.

Fix `nightly_build.yml` to upload `artifacts/ghafmt-*` instead of
`artifacts/*` to the nightly release, excluding the Docker layer-cache
artifact that would otherwise appear as a spurious release asset. (e104571)

### Dependencies

- chore(deps): update taiki-e/install-action action to v2.69.7 (b52b460)
- chore(deps): update taiki-e/install-action action to v2.69.8 (cb38611)

## 0.1.0 (2026-03-23)

### Features

#### Action metadata file formatting

`ghafmt` now detects and formats GitHub Actions action metadata files
(`action.yml` / `action.yaml`) in addition to workflow files.

##### Supported action types

- **Composite actions** (`runs.using: composite`) — top-level key ordering,
  `runs` key ordering, step key ordering, alphabetical `with` sorting inside
  steps, alphabetical `inputs`/`outputs` sorting with idiomatic per-entry key
  ordering, `branding` key ordering, and snake_case enforcement on step IDs
- **JavaScript actions** (`runs.using: node20` / `node24`) — top-level key
  ordering, `runs` key ordering (`using → pre → pre-if → main → post →
  post-if`), alphabetical `inputs`/`outputs` sorting
- **Docker container actions** (`runs.using: docker`) — top-level key
  ordering, `runs` key ordering (`using → image → args → env →
  pre-entrypoint → entrypoint → post-entrypoint`), alphabetical
  `inputs`/`outputs` sorting

Files that cannot be identified as a workflow or action pass through
with presentation transforms only (blank lines, variable spacing).

#### Cross-platform binary builds

Pre-built binaries for `aarch64-apple-darwin` and `x86_64-unknown-linux-musl`
are now attached to each GitHub Release as assets. Support for
`aarch64-unknown-linux-musl` and `x86_64-pc-windows-msvc` is planned for
future releases. (247bd28)

#### Docker image published to GHCR

`ghafmt` is now available as a Docker image on GitHub Container Registry:

```
docker pull ghcr.io/jonathanrainer/ghafmt:latest
```

Nightly builds are also published as `ghcr.io/jonathanrainer/ghafmt:nightly`. (097523e)

#### GitHub Action

`ghafmt` is now available as a composite GitHub Action:

```yaml
- uses: jonathanrainer/ghafmt@v0.2.0
  with:
    path: .github/workflows/
    mode: check
```

Supports Linux x86_64, Linux ARM64, and macOS ARM64 and Intel. (90ca03b)

#### macOS Intel binary builds

Pre-built binaries for `x86_64-apple-darwin` (macOS Intel) are now included in
each GitHub Release, cross-compiled from the ARM64 runner using Apple's clang. (839d6cb)

#### macOS binary signing and notarization

Released macOS binaries are now signed with a Developer ID Application certificate and notarized
with Apple, so Gatekeeper no longer blocks them on download.

Signing and notarization is implemented as a composite action
(`.github/actions/sign-and-notarize`) following the existing `set-cargo-version` pattern, and
is called from the build artifacts workflow for macOS targets only. It runs exclusively on `main`
so signing credentials are never exposed during PR or feature branch builds. (638ec79)

#### Refactor: split `main.rs`, improve CLI, harden stdin reading

##### Structural changes

- `main.rs` has been split into focused modules:
    - `src/lib.rs` — orchestration and `FormatterResult`
    - `src/cli.rs` — Clap argument definitions
    - `src/commands/` — one file per mode (`check`, `format`, `write`, `list`), sharing a `Command` trait
    - `src/fs.rs` — `expand_paths`, `atomic_write`, stdin reading

##### Bug fixes

- `read_from_stdin` previously used `take(LIMIT)` to cap reads but then checked `n > LIMIT`, a guard that could never
  fire. The function now probes for an overflow byte after reading, correctly detecting inputs that exceed the limit
  while accepting files of exactly the limit size.

##### Dependencies

- Added `atomic-write-file` — replaces the hand-rolled temp-file-then-rename implementation of `atomic_write`.
- Added `patharg` — typed `InputArg` enum wrapping file paths and stdin (`-`).
- Added `strum_macros` — derive-based enum string conversions for CLI mode/colour variants.

##### Tests

- Added unit tests for `read_with_limit` (under limit, exactly at limit, over limit) and `atomic_write` (overwrites
  rather than appends). (809abb2)

### Fixes

#### Acknowledgements, licence detection fix & changeset detection

Added an Acknowledgements section to the README crediting [@pantoniou](https://github.com/pantoniou)
for libfyaml and [@0k](https://github.com/0k) for the fyaml Rust bindings.

Added a `LICENSE` symlink pointing to `LICENSE-MIT` so that GitHub's licence detector
correctly identifies the repository licence rather than reporting it as unknown.

Updated the pre-commit hooks to detect if a changeset exists on the branch,
just to save time forgetting one on PRs (3f8595e)

#### Pre-commit hooks

A `.pre-commit-config.yaml` is now provided for local development. It runs
clippy, cargo deny, actionlint, and ghafmt (from source) as opt-in pre-commit
checks. (ce899c8)

#### Add README, CONTRIBUTING, and MIT/Apache-2.0 dual license

Adds a README with usage examples, CI integration snippets, and a before/after
example. Adds CONTRIBUTING.md with dev setup and transformer guide. Switches
license from GPL-3.0-only to MIT OR Apache-2.0. (490bf69)

#### Bring in libfyaml comment-handling fixes and related Rust improvements

Integrates fixes from the restructured `jonathanrainer/libfyaml` fork and
adds the Rust-side presentation work that depends on them.

##### Features

- Add `TopLevelCommentSpacer` presentation transformer: inserts a blank line
  before any block of standalone top-level comments (col 0) that follows
  content at a deeper indentation level, covering end-of-file comments and
  other top-level comment blocks not preceded by a known top-level key

##### Bug fixes

- `insert_blank_line_before_comment_block` gains a `max_comment_indent`
  parameter; when set, a comment preceded by an indent deeper than the
  threshold is treated as content rather than part of the preceding comment
  block, preventing the scan from skipping deeply-nested comments that belong
  to the previous section
- Fix `TopLevelCommentSpacer` not triggering on inline right-hand comments:
  only inserts a blank line when the preceding event is a `Linebreak`, so
  comments that appear on the same line as a value are correctly ignored (7d783e2)

#### Code review fixes and improvements

Address findings from a review of the codebase.

##### Bug fixes

- Fix typo in `FilterSorter`: `on/pull_request_targe/branches-ignore` →
  `on/pull_request_target/branches-ignore`, which was silently skipping that
  sort path
- Remove `outputs` and `secrets` from `WorkflowDispatchSorter`; those keys
  belong to `workflow_call` only and were being incorrectly sorted
- Fix `Rename::Ord`/`PartialOrd`/`Eq`/`PartialEq` impls in `CaseEnforcer` that
  compared only depth, violating the `Eq`/`Ord` contract; sorting is now done
  via `sort_by_key` with `Reverse` on depth
- Reject `stdin (-)` with `--mode=list`, consistent with the existing
  `--mode=write` restriction; produces a structured error via the same
  `StdinCannotBeUsedWithList` variant

##### Refactoring

- Replace ad-hoc `eprintln!` calls for early validation errors in `Ghafmt::run`
  with structured `Error` variants rendered through `render_error`, so colour
  mode is respected and diagnostics are consistent
- Extract `build_handler` and `render_error` as `pub(crate)` free functions;
  remove them from the `Command` trait entirely — callers use the free functions
  directly
- Simplify `FormatterResult`: replace `original: Option<String>` + accessor
  method with a plain `pub(crate) original: String` field
- Change `Ghafmt::run` signature from `mut self` to `&mut self`
- Pre-compute static key-ordering `Vec<String>` fields in transformer struct
  constructors instead of allocating on every `process()` call; affected
  transformers: `TopLevelSorter`, `JobSorter`, `StepSorter`, `ConcurrencySorter`,
  `StrategySorter`, `ContainerSorter`, `RunsOnSorter`, `EnvironmentSorter`,
  `WorkflowCallSorter`
- Change `sort_mapping_at_path` and `classify_renames` signatures to accept
  `&[T]` instead of `&Vec<T>`
- Make the `WorkflowProcessor` transformer pipeline injectable via
  `WorkflowProcessor::new(Vec<Box<dyn StructureTransformer>>)` with a
  `Default` impl that builds the standard pipeline; removes the need for a
  test-only constructor
- Derive `Copy` on `State` enums in `JobsBlankLines` and `StepsBlankLines`;
  remove now-redundant `.clone()` calls at match sites
- Remove stale `#[allow(unused_assignments)]` from `errors.rs`
- Fix `--colour` flag spelling in the CLI integration test (was `--color`,
  causing the test to pass vacuously against a clap parse error)

##### Tests

- Add unit tests for `source_window` and `line_col_to_byte_offset` in `errors.rs`
- Add integration tests for `WorkflowProcessor` error recovery: failed
  transformer emits a warning, document is restored, and subsequent transformers
  still run
- Add `list_mode_rejects_stdin` CLI integration test
- Add `pull_request_target_branches_ignore_sorted` regression test in `FilterSorter`
- Remove duplicate `multi_job_comments_between_steps` test case in `StepsBlankLines` (5fd7503)

#### Fix `--check` silent failure and multi-file error message

`--check` now prints a unified diff to stderr for each file that differs from its
formatted form, instead of silently exiting 1. The no-flag multi-file error message
now mentions `--check` and `--list` alongside `--write`. (4d2689b)

#### Fix Docker build failure due to missing musl headers

The Docker build was failing because `bindgen` (used by `fyaml-sys`) invoked clang with the
default glibc include paths, causing a fatal error when it couldn't find
`bits/libc-header-start.h`.

Fixed by setting `BINDGEN_EXTRA_CLANG_ARGS=--sysroot=/usr/local/musl/${RUST_MUSL_CROSS_TARGET}`
in the `cargo build` step, pointing clang at the musl sysroot already bundled in the
`rust-musl-cross` image. This works for both `linux/amd64` and `linux/arm64` targets. (f569f76)

#### Fix folded scalars emitting extra blank lines

When a string value used a `>`/`>-`/`>+` folded scalar an extra trailing blank line
would be emitted after it.

This was due to an underlying bug in the YAML parsing library (`libfyaml`) which
is now resolved. (062854a)

#### Fix GitHub licence detection

Replaced the `LICENSE` symlink with a real file (renamed from `LICENSE-MIT`) so that
GitHub's licence detector correctly identifies the repository as MIT without reporting
duplicates. `LICENSE-APACHE` is retained for dual-licence completeness. (fffec1f)

#### Fix case enforcement clobbering YAML literal block scalars

When a string value used a `|` block scalar (e.g. multi-line `${{ }}` expressions),
`set_yaml_at` would silently fold newlines to spaces instead of failing, causing the
updated value to be written as a single-line string. The enforcer now detects literal
block style up front and formats the replacement value as a proper `|` block scalar
before writing. (1d769c1)

#### Fix literal keep (`|+`) scalars producing double blank lines

When a step used a `|+` (literal keep) block scalar, formatting would insert an extra
blank line after it on every pass, breaking idempotency.

The underlying `libfyaml` bug that caused `|+` to emit a spurious extra trailing newline
was fixed upstream (PR #268). However the blank-line insertion logic in ghafmt was not
accounting for the blank line already present in the event stream from the keep-chomping
semantics, so it would add another on top.

The fix detects when two consecutive line-break events already exist at the insertion
point and skips insertion in that case. (6f07276)

#### Cleaner nightly version strings

Nightly builds now report `nightly.YYYYMMDD.SHORTSHA` from `--version` instead of the previous
`0.0.0-nightly.YYYYMMDD.SHORTSHA`. Nightly artifact filenames follow the same format. (088fed0)

#### Security audit remediation

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
  in `deny.toml` (29d5880)

#### Use install-action for cargo-deny in CI

Switch the `deny` CI job from `EmbarkStudios/cargo-deny-action` to
`taiki-e/install-action`, fetching a pre-compiled binary instead of
compiling from source on every run. (deac02e)

### Project Infrastructure

- Add rust-toolchain.toml as single source of truth for Rust version (28b49be)
- Migrate Renovate config (96edec2)

#### Add CI infrastructure

Sets up the full CI pipeline and release tooling for the project:

- **Code checks** (`code_checks.yml`): nightly `rustfmt`, `clippy`, `cargo test` (with `actionlint` installed), and
  `cargo deny` for security advisories
- **Workflow checks** (`workflow_checks.yml`): `actionlint` via Docker image and `ghafmt --check` to keep workflow files
  formatted
- **Dependency updates** (`renovate.json`): Renovate configured to update GitHub Actions SHA pins, Cargo dependencies,
  and the pinned Rust toolchain version
- **Changeset tracking** (`knope.toml`): `knope create-changeset` workflow for documenting changes per PR

Also fixes several pre-existing issues uncovered by the new checks: invalid `GPL-3` SPDX identifier, pending tests not
being skipped in CI, failing `actionlint` tests caused by unresolvable reusable workflow references, and `clippy`
warnings in integration test helpers. (8f871f7)

#### Milestone automation in release workflow

Releases now require a GitHub milestone titled `v{version}` (e.g. `v0.1.0`)
to exist before the release workflow will proceed. Once the release is
published the milestone is closed automatically. (664180a)

#### Nightly binary builds

A nightly GitHub pre-release is published each night at
`https://github.com/jonathanrainer/ghafmt/releases/tag/nightly` with
pre-built binaries for all four supported platforms. Nightly binaries
report their build date and commit in `--version` output. (247bd28)

#### Manual trigger for nightly build workflow

The nightly build workflow can now be triggered manually via
`workflow_dispatch` in addition to the scheduled nightly run. (882b45c)

#### Fix prepare-release PR title format

The PR and commit created by `prepare-release` now use
`chore: Prepare To Release v$version` instead of
`chore: prepare release $version`. (cfe4dd9)

#### Fix Renovate changeset deduplication

Renovate changesets are now named `renovate_pr_<number>.md` instead of deriving a slug from
the branch name. The old slug-based approach silently skipped changeset creation when a file
for the same package already existed from a previous PR, meaning repeated dependency bumps
went undocumented. Using the PR number guarantees a unique file per PR. (96edec2)

#### Pass secrets through to the build artifacts reusable workflow

Added `secrets: inherit` to both callers of `_build_artifacts.yml` (`nightly_build.yml` and
`release.yml`) so that repository secrets are available inside the reusable workflow. (ca26e4b)

#### cargo-edit is prohibited from downgrading a version, therefore we have to us `yq` in TOML

mode so we can support our nightly builds (4092bab)

#### Harden repository security ahead of going public

Added `CODEOWNERS` to auto-request review on all PRs. Tightened workflow permissions in
`release.yml` and `prepare_release.yml` by moving broad top-level `permissions` blocks down
to per-job level, so each job holds only the minimum permissions it needs. (796104d)

#### Pin action example to SHA and automate updates on release

The GitHub Action usage example in the README now pins to a full commit SHA rather than a
floating version tag, following the recommended security practice for GitHub Actions:

```yaml
- uses: jonathanrainer/ghafmt@1cc43c68845e56ea46b7a9c6017e024283081648
```

The `prepare-release` workflow in `knope.toml` now automatically updates this pin to the correct
SHA and version comment (e.g. `# v0.3.0`) as part of the release prep commit, so the docs stay
accurate across releases.

Also suppresses a Dockerfile linter warning about `--platform=linux/amd64`, with an explanatory
comment clarifying the intent. (4237b9d)

#### Auto-generate changesets for Renovate PRs

Adds a `renovate_changeset.yml` workflow that automatically commits a `.changeset/*.md` file
to every Renovate PR, preventing CI failures caused by missing changesets. The workflow
conditions on the PR author (`github.event.pull_request.user.login`) rather than the event
actor so that manually closing and reopening a PR still triggers changeset generation.

Also adds a `Dependencies` changelog section to `knope.toml` so dependency updates appear
under their own heading in the CHANGELOG. (8f871f7)

#### Set Renovate to update dependencies in real time

Removes the default weekly schedule so Renovate raises PRs as soon as updates are detected. (8f871f7)

### Dependencies

- Update Rust crate tracing-subscriber to v0.3.23 (96edec2)
- Update taiki-e/install-action action to v2.68.33 (originally merged
  in [#28](https://github.com/jonathanrainer/ghafmt/pull/28)) (96edec2)
- Update Rust crate assert_cmd to v2.2.0 (96edec2)
- Update Rust crate clap to v4.6.0 (96edec2)
- Update actions/checkout action to v6 (96edec2)
- chore(deps): update dependency rust-lang/rust to v1.94.0 (96edec2)
- chore(deps): update taiki-e/install-action action to v2.68.34 (96edec2)
- chore(deps): update taiki-e/install-action action to v2.68.35 (96edec2)
- chore(deps): update taiki-e/install-action action to v2.69.1 (ecb3daa)
- chore(deps): update taiki-e/install-action action to v2.69.2 (e8bc0cc)
- chore(deps): update taiki-e/install-action action to v2.69.5 (78ffcf8)
- chore(deps): update taiki-e/install-action action to v2.69.6 (9cc85d9)
