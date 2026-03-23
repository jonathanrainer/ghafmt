---
default: minor
---

# Action metadata file formatting

`ghafmt` now detects and formats GitHub Actions action metadata files
(`action.yml` / `action.yaml`) in addition to workflow files.

#### Supported action types

- **Composite actions** (`runs.using: composite`) — top-level key ordering,
  `runs` key ordering, step key ordering, alphabetical `with` sorting inside
  steps, alphabetical `inputs`/`outputs` sorting with idiomatic per-entry key
  ordering, `branding` key ordering, and snake_case enforcement on step IDs
- **JavaScript actions** (`runs.using: node20` / `node24`) — top-level key
  ordering, `runs` key ordering (`using → pre → pre-if → main → post →
  post-if`), alphabetical `inputs`/`outputs` sorting
- **Docker container actions** (`runs.using: docker`) — top-level key
  ordering, `runs` key ordering (`using → image → args → env →
  pre-entrypoint → entrypoint → post-entrypoint`), alphabetical
  `inputs`/`outputs` sorting

Files that cannot be identified as a workflow or action pass through
with presentation transforms only (blank lines, variable spacing).

#### Top-level key ordering

Action metadata files are sorted into the canonical order recommended
by the GitHub Actions documentation:
`name → description → author → inputs → outputs → runs → branding`