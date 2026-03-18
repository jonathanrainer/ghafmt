---
default: patch
---

# Use install-action for cargo-deny in CI

Switch the `deny` CI job from `EmbarkStudios/cargo-deny-action` to
`taiki-e/install-action`, fetching a pre-compiled binary instead of
compiling from source on every run.