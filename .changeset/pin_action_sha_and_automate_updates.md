---
default: internal
---

# Pin action example to SHA and automate updates on release

The GitHub Action usage example in the README now pins to a full commit SHA rather than a
floating version tag, following the recommended security practice for GitHub Actions:

```yaml
- uses: jonathanrainer/ghafmt@1cc43c68845e56ea46b7a9c6017e024283081648
```

The `prepare-release` workflow in `knope.toml` now automatically updates this pin to the correct
SHA and version comment (e.g. `# v0.3.0`) as part of the release prep commit, so the docs stay
accurate across releases.

Also suppresses a Dockerfile linter warning about `--platform=linux/amd64`, with an explanatory
comment clarifying the intent.