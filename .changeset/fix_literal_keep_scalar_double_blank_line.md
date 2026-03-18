---
default: patch
---

# Fix literal keep (`|+`) scalars producing double blank lines

When a step used a `|+` (literal keep) block scalar, formatting would insert an extra
blank line after it on every pass, breaking idempotency.

The underlying `libfyaml` bug that caused `|+` to emit a spurious extra trailing newline
was fixed upstream (PR #268). However the blank-line insertion logic in ghafmt was not
accounting for the blank line already present in the event stream from the keep-chomping
semantics, so it would add another on top.

The fix detects when two consecutive line-break events already exist at the insertion
point and skips insertion in that case.