---
default: patch
---

# Fix case enforcement clobbering YAML literal block scalars

When a string value used a `|` block scalar (e.g. multi-line `${{ }}` expressions),
`set_yaml_at` would silently fold newlines to spaces instead of failing, causing the
updated value to be written as a single-line string. The enforcer now detects literal
block style up front and formats the replacement value as a proper `|` block scalar
before writing.
