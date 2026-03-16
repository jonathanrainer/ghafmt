---
default: internal
---

# Auto-generate changesets for Renovate PRs

Adds a `renovate_changeset.yml` workflow that automatically commits a `.changeset/*.md` file
to every Renovate PR, preventing CI failures caused by missing changesets. The workflow
conditions on the PR author (`github.event.pull_request.user.login`) rather than the event
actor so that manually closing and reopening a PR still triggers changeset generation.

Also adds a `Dependencies` changelog section to `knope.toml` so dependency updates appear
under their own heading in the CHANGELOG.
