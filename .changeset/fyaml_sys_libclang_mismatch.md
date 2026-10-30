---
default: dependencies
---

Fix build on Linux when multiple clang versions are installed

Bump the `fyaml-sys` fork so bindgen no longer scrapes include paths from a
`clang` binary that may differ from the libclang it loads. This broke the
build on Ubuntu 26.04, which ships clang 20, 21 and 22.