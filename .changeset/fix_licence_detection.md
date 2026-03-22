---
default: patch
---

# Fix GitHub licence detection

Replaced the `LICENSE` symlink with a real file (renamed from `LICENSE-MIT`) so that
GitHub's licence detector correctly identifies the repository as MIT without reporting
duplicates. `LICENSE-APACHE` is retained for dual-licence completeness.