---
default: internal
---

# Auto-generate changesets for Renovate PRs

Adds a `renovate_changeset.yml` workflow that automatically commits a `.changeset/*.md` file
to every Renovate PR, preventing CI failures caused by missing changesets.

Also adds a `Dependencies` changelog section to `knope.toml` so dependency updates appear
under their own heading in the CHANGELOG.
