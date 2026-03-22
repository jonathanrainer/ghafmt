---
default: internal
---

# Harden repository security ahead of going public

Added `CODEOWNERS` to auto-request review on all PRs. Tightened workflow permissions in
`release.yml` and `prepare_release.yml` by moving broad top-level `permissions` blocks down
to per-job level, so each job holds only the minimum permissions it needs.