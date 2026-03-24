---
default: patch
---

# Fix step ID references in bare `if:` conditions not being updated

When a step ID was renamed (e.g. `cache-thing` → `cache_thing`), references
to that ID in `if:` conditions written without `${{ }}` delimiters were left
unchanged, producing a broken workflow.

GitHub Actions allows `if:` fields to omit the expression wrapper, so
`if: steps.cache-thing.outputs.cache-hit != 'true'` is valid. The reference
updater now processes `/if` paths regardless of whether they contain `${{`.