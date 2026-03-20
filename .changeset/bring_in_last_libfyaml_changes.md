---
default: patch
---

# Bring in libfyaml comment-handling fixes and related Rust improvements

Integrates fixes from the restructured `jonathanrainer/libfyaml` fork and
adds the Rust-side presentation work that depends on them.

## Features
- Add `TopLevelCommentSpacer` presentation transformer: inserts a blank line
  before any block of standalone top-level comments (col 0) that follows
  content at a deeper indentation level, covering end-of-file comments and
  other top-level comment blocks not preceded by a known top-level key

## Bug fixes
- `insert_blank_line_before_comment_block` gains a `max_comment_indent`
  parameter; when set, a comment preceded by an indent deeper than the
  threshold is treated as content rather than part of the preceding comment
  block, preventing the scan from skipping deeply-nested comments that belong
  to the previous section
- Fix `TopLevelCommentSpacer` not triggering on inline right-hand comments:
  only inserts a blank line when the preceding event is a `Linebreak`, so
  comments that appear on the same line as a value are correctly ignored