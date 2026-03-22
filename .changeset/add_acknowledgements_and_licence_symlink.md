---
default: patch
---

# Acknowledgements, licence detection fix & changeset detection

Added an Acknowledgements section to the README crediting [@pantoniou](https://github.com/pantoniou)
for libfyaml and [@0k](https://github.com/0k) for the fyaml Rust bindings.

Added a `LICENSE` symlink pointing to `LICENSE-MIT` so that GitHub's licence detector
correctly identifies the repository licence rather than reporting it as unknown.

Updated the pre-commit hooks to detect if a changeset exists on the branch,
just to save time forgetting one on PRs
