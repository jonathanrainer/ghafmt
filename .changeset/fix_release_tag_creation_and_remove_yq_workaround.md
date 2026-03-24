---
default: internal
---

# Fix release tag creation and remove yq workaround

Use a fine-grained PAT (`RELEASE_PAT`) for the knope release step so that
tag creation is not blocked by the repository ruleset restricting ref
creation for the default `GITHUB_TOKEN`.

Remove the `set-cargo-version` yq TOML-output workaround now that the
`ubuntu-24.04-arm` runner image ships a yq version with full TOML support.
