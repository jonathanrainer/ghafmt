---
default: patch
---

# Acknowledgements and licence detection fix

Added an Acknowledgements section to the README crediting [@pantoniou](https://github.com/pantoniou)
for libfyaml and [@0k](https://github.com/0k) for the fyaml Rust bindings.

Added a `LICENSE` symlink pointing to `LICENSE-MIT` so that GitHub's licence detector
correctly identifies the repository licence rather than reporting it as unknown.