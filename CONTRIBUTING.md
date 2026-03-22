# Contributing to ghafmt

## Prerequisites

- **Rust stable** (see `rust-toolchain.toml` for the pinned version) — install via [rustup](https://rustup.rs)
- **Rust nightly** with `rustfmt` (for `cargo +nightly fmt`): `rustup toolchain install nightly --component rustfmt`
- **pre-commit** (optional, for local hook runner): `pip install pre-commit`
- **actionlint** (required for the test suite): see [installation instructions](https://github.com/rhysd/actionlint#installation)

## Dev setup

```bash
git clone https://github.com/jonathanrainer/ghafmt
cd ghafmt
pre-commit install   # optional but recommended
cargo test
```

## Running tests

```bash
cargo test
```

The test suite includes:

- **Roundtrip fixture tests** — each file in `tests/fixtures/dirty/` is formatted and compared against its counterpart in `tests/fixtures/clean/`
- **Idempotency tests** — every clean fixture is formatted again and must be unchanged
- **actionlint tests** — clean workflow fixtures are validated with `actionlint` to ensure the formatter doesn't produce invalid workflow YAML (action metadata fixtures are excluded, as actionlint does not support them)
- **CLI integration tests** — `assert_cmd`-based tests covering `--mode=check`, `--mode=write`, `--mode=list`, and stdin

## Adding a formatting rule

Rules fall into two categories depending on whether they change document structure or only presentation.

### Structure transformers

These operate on the parsed YAML tree (reordering keys, sorting arrays, etc.).

1. Create `src/structure_transformers/<name>.rs` implementing the transformer logic
2. Add `pub mod <name>;` to `src/structure_transformers/mod.rs`
3. Register the transformer in the relevant document-type pipeline(s) in `src/lib.rs` (`get_transformers`)
4. Add a dirty/clean fixture pair under the appropriate subdirectory in `tests/fixtures/`:
   - Workflow rules: `tests/fixtures/dirty/<name>.yaml` / `tests/fixtures/clean/<name>.yaml`
   - Action metadata rules: `tests/fixtures/dirty/actions/<name>.yaml` / `tests/fixtures/clean/actions/<name>.yaml`

### Presentation transformers

These operate on the emitted string (inserting blank lines, adjusting whitespace, etc.).

1. Create `src/presentation_transformers/<name>.rs` implementing the transformer logic
2. Add `pub mod <name>;` to `src/presentation_transformers/mod.rs`
3. Register the transformer in the relevant document-type pipeline(s) in `src/lib.rs` (`get_transformers`)
4. Add a dirty/clean fixture pair as above

## Changeset requirement

Every PR must include a changeset describing what changed and its semver impact. Create `.changeset/<descriptive_name>.md`:

```markdown
---
default: patch
---

Brief description of the change.
```

Use `patch` for bug fixes, `minor` for new features, `major` for breaking changes.

## PR checklist

- [ ] `cargo test` passes
- [ ] Changeset added in `.changeset/`
- [ ] `cargo deny check` passes
- [ ] `cargo +nightly fmt --all` applied
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean