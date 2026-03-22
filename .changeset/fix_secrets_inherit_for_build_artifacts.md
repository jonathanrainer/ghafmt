---
default: internal
---

# Pass secrets through to the build artifacts reusable workflow

Added `secrets: inherit` to both callers of `_build_artifacts.yml` (`nightly_build.yml` and
`release.yml`) so that repository secrets are available inside the reusable workflow.