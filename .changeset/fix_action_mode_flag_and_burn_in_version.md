---
default: patch
---

# Store version in the action

Remove the `version` input and instead hardcode `ACTION_VERSION` as an env var
in the download step, kept in lockstep with Cargo.toml via a new Knope step
that updates it automatically during release preparation.