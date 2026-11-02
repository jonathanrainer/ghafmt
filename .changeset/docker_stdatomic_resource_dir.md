---
default: internal
---

Fix Docker image build failing to find `stdatomic.h`

Pass clang's resource dir to bindgen in the Dockerfile, as libclang can no
longer locate its builtin headers in the musl-cross image now that `fyaml-sys`
skips include path detection on Linux.