---
default: patch
---

# Fix Docker build failure due to missing musl headers

The Docker build was failing because `bindgen` (used by `fyaml-sys`) invoked clang with the
default glibc include paths, causing a fatal error when it couldn't find
`bits/libc-header-start.h`.

Fixed by setting `BINDGEN_EXTRA_CLANG_ARGS=--sysroot=/usr/local/musl/${RUST_MUSL_CROSS_TARGET}`
in the `cargo build` step, pointing clang at the musl sysroot already bundled in the
`rust-musl-cross` image. This works for both `linux/amd64` and `linux/arm64` targets.
