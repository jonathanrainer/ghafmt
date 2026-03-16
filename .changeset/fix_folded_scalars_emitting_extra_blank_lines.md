---
default: patch
---

# Fix folded scalars emitting extra blank lines

When a string value used a `>`/`>-`/`>+` folded scalar an extra trailing blank line
would be emitted after it.

This was due to an underlying bug in the YAML parsing library (`libfyaml`) which
is now resolved.
