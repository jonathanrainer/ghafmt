---
default: minor
---

# GitHub Action

`ghafmt` is now available as a composite GitHub Action:

```yaml
- uses: jonathanrainer/ghafmt@v0.2.0
  with:
    path: .github/workflows/
    mode: check
```

Supports Linux x86_64, Linux ARM64, and macOS ARM64 and Intel.