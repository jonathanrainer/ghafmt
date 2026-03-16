---
default: internal
---

# Add CI infrastructure

Sets up the full CI pipeline and release tooling for the project:

- **Code checks** (`code_checks.yml`): nightly `rustfmt`, `clippy`, `cargo test` (with `actionlint` installed), and `cargo deny` for security advisories
- **Workflow checks** (`workflow_checks.yml`): `actionlint` via Docker image and `ghafmt --check` to keep workflow files formatted
- **Dependency updates** (`renovate.json`): Renovate configured to update GitHub Actions SHA pins, Cargo dependencies, and the pinned Rust toolchain version
- **Changeset tracking** (`knope.toml`): `knope create-changeset` workflow for documenting changes per PR

Also fixes several pre-existing issues uncovered by the new checks: invalid `GPL-3` SPDX identifier, pending tests not being skipped in CI, failing `actionlint` tests caused by unresolvable reusable workflow references, and `clippy` warnings in integration test helpers.
