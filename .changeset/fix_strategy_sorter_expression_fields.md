---
default: patch
---

# Fix strategy-sorter warning on expression-valued matrix fields

`strategy-sorter` now silently skips `include`, `exclude`, and dimension
keys whose values are GitHub Actions expressions (e.g.
`${{ fromJSON(inputs.platforms) }}`), rather than emitting a
`TypeMismatch: expected sequence, got non-sequence` warning.

Fixes #88.