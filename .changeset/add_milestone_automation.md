---
default: internal
---

# Milestone automation in release workflow

Releases now require a GitHub milestone titled `v{version}` (e.g. `v0.1.0`)
to exist before the release workflow will proceed. Once the release is
published the milestone is closed automatically.
