---
default: patch
---

# Fix `--check` silent failure and multi-file error message

`--check` now prints a unified diff to stderr for each file that differs from its
formatted form, instead of silently exiting 1. The no-flag multi-file error message
now mentions `--check` and `--list` alongside `--write`.
