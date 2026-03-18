---
default: patch
---

# Pre-commit hooks

A `.pre-commit-config.yaml` is now provided for local development. It runs
clippy, cargo deny, actionlint, and ghafmt (from source) as opt-in pre-commit
checks.