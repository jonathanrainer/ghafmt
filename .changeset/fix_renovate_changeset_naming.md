---
default: internal
---

# Fix Renovate changeset deduplication

Renovate changesets are now named `renovate_pr_<number>.md` instead of deriving a slug from
the branch name. The old slug-based approach silently skipped changeset creation when a file
for the same package already existed from a previous PR, meaning repeated dependency bumps
went undocumented. Using the PR number guarantees a unique file per PR.
