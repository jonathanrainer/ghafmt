---
default: internal
---

cargo-edit is prohibited from downgrading a version, therefore we have to us `yq` in TOML
mode so we can support our nightly builds
