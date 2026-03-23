---
default: internal
---

# Harden workflows ahead of making repository public

Add explicit `permissions: contents: read` to all read-only jobs in
`code_checks.yml`, `workflow_checks.yml`, and the `setup` job in
`nightly_build.yml`, so that a default-permissions change cannot silently
grant write access.

Fix `nightly_build.yml` to upload `artifacts/ghafmt-*` instead of
`artifacts/*` to the nightly release, excluding the Docker layer-cache
artifact that would otherwise appear as a spurious release asset.