---
default: internal
---

# Regenerate Cargo.lock during prepare-release

Added a `cargo update -p ghafmt` step to the `prepare-release` workflow so
that `Cargo.lock` reflects the new version before the release commit is made.
