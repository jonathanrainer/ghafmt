# ghafmt

A formatter for GitHub Actions workflow and action metadata files.

[![CI](https://github.com/jonathanrainer/ghafmt/actions/workflows/code_checks.yml/badge.svg)](https://github.com/jonathanrainer/ghafmt/actions/workflows/code_checks.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

`ghafmt` enforces a consistent style across your GitHub Actions YAML files — both workflow files and action metadata
files (`action.yml` / `action.yaml`). It reorders keys, sorts blocks alphabetically, inserts blank lines between
steps and jobs, and converts IDs to `snake_case` — so code review diffs show only meaningful changes, not formatting
noise.

**Workflow formatting rules:**

- Step keys reordered to: `name` → `uses`/`run` → `id` → `with`/`env`
- Top-level workflow keys sorted (`name` → `on` → `env` → `jobs`)
- Trigger events, `needs` arrays, `runs-on` arrays, and filter arrays sorted alphabetically
- Blank lines inserted between top-level keys, jobs, and steps
- Job IDs and step IDs converted to `snake_case`
- Keys within `with`, `env`, `permissions`, and similar maps sorted alphabetically

**Action metadata formatting rules (`action.yml` / `action.yaml`):**

- Top-level keys sorted to canonical order: `name` → `description` → `author` → `inputs` → `outputs` → `runs` → `branding`
- `inputs` and `outputs` sorted alphabetically; per-entry keys sorted idiomatically
- `runs` keys sorted by action type: composite (`using` → `steps`), JavaScript (`using` → `pre` → `pre-if` → `main` → `post` → `post-if`), Docker (`using` → `image` → `args` → `env` → `pre-entrypoint` → `entrypoint` → `post-entrypoint`)
- Step keys and `with` maps inside composite action steps sorted alphabetically
- Step IDs converted to `snake_case`
- `branding` keys sorted (`icon` → `color`)

## Before / After

**Before:**

```yaml
on:
  workflow_dispatch:
  push:
    branches: [main]
  pull_request:
name: CI Pipeline
jobs:
  RunTests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        name: Checkout
      - with:
          node-version: '20'
          cache: npm
        uses: actions/setup-node@v4
        name: Setup Node
      - run: npm install
        name: Install deps
        id: install
      - env:
          NODE_ENV: test
          CI: true
        run: npm test
        name: Run tests
        id: testStep
  BuildAndDeploy:
    needs: [RunTests]
    runs-on: ubuntu-latest
    steps:
      - name: Build
        run: npm run build
      - env:
          AWS_SECRET_ACCESS_KEY: ${{ secrets.SECRET_KEY }}
          AWS_ACCESS_KEY_ID: ${{ secrets.ACCESS_KEY }}
          AWS_REGION: us-east-1
        run: aws s3 sync dist/ s3://my-bucket
        name: Deploy
        id: deployStep
```

**After:**

```yaml
name: CI Pipeline

on:
  pull_request:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  run_tests:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          cache: npm
          node-version: '20'

      - name: Install deps
        id: install
        run: npm install

      - name: Run tests
        id: test_step
        run: npm test
        env:
          CI: true
          NODE_ENV: test

  build_and_deploy:
    needs: [run_tests]
    runs-on: ubuntu-latest
    steps:
      - name: Build
        run: npm run build

      - name: Deploy
        id: deploy_step
        run: aws s3 sync dist/ s3://my-bucket
        env:
          AWS_ACCESS_KEY_ID: ${{ secrets.ACCESS_KEY }}
          AWS_REGION: us-east-1
          AWS_SECRET_ACCESS_KEY: ${{ secrets.SECRET_KEY }}
```

## Installation

### Pre-built binary

Download the latest release from [GitHub Releases](https://github.com/jonathanrainer/ghafmt/releases/latest):

```bash
# Replace <VERSION> and <TARGET> with values from the releases page:
#   x86_64-unknown-linux-musl  (Linux x86_64)
#   aarch64-unknown-linux-musl (Linux ARM64)
#   aarch64-apple-darwin       (macOS Apple Silicon)
#   x86_64-apple-darwin        (macOS Intel)
curl -fsSL https://github.com/jonathanrainer/ghafmt/releases/download/v<VERSION>/ghafmt-<VERSION>-<TARGET>.tar.gz | tar -xz
sudo mv ghafmt /usr/local/bin/
```

### Docker

```bash
docker pull ghcr.io/jonathanrainer/ghafmt:latest
```

## Usage

```bash
# Format to stdout
ghafmt workflow.yml

# Read from stdin
cat workflow.yml | ghafmt -

# Check formatting without writing (exits non-zero if any file is dirty)
ghafmt --mode=check .github/workflows/

# Write changes in place
ghafmt --mode=write .github/workflows/

# List files that would be changed
ghafmt --mode=list .github/workflows/
```

### Docker

```bash
docker run --rm -v "$PWD":/work ghcr.io/jonathanrainer/ghafmt:latest --mode=check /work/.github/workflows/
```

## CI Integration

### GitHub Actions

Use the bundled action — it downloads the correct pre-built binary for the runner platform and automatically
discovers both workflow files and any `action.yml`/`action.yaml` files in the repository:

```yaml
- uses: jonathanrainer/ghafmt@92092637edea05c6d3f91b5409b744b74c353dbd # v0.1.3
  with:
    mode: check          # check (default) | write | list
    path: .github/workflows/
```

Or use the Docker image directly:

```yaml
- name: Check workflow formatting
  run: |
    docker run --rm -v "$GITHUB_WORKSPACE":/work \
      ghcr.io/jonathanrainer/ghafmt:latest --mode=check /work/.github/workflows/
```

### CircleCI

```yaml
- run:
    name: Check workflow formatting
    command: |
      VERSION=$(curl -fsSL https://api.github.com/repos/jonathanrainer/ghafmt/releases/latest | grep tag_name | cut -d'"' -f4 | sed 's/^v//')
      curl -fsSL "https://github.com/jonathanrainer/ghafmt/releases/download/v${VERSION}/ghafmt-${VERSION}-x86_64-unknown-linux-musl.tar.gz" | tar -xz -C /tmp
      /tmp/ghafmt --mode=check .github/workflows/
```

## Pre-commit

Add to your `.pre-commit-config.yaml` (requires `ghafmt` on `PATH`):

```yaml
repos:
  - repo: local
    hooks:
      - id: ghafmt
        name: ghafmt
        language: system
        entry: ghafmt --mode=check
        pass_filenames: true
        files: ^(\.github/workflows/.*\.ya?ml|(.*\/)?action\.ya?ml)$
```

## Acknowledgements

`ghafmt` is built on top of two foundational projects:

- [**libfyaml**](https://github.com/pantoniou/libfyaml) by [@pantoniou](https://github.com/pantoniou) — the YAML parser and emitter at the core of this tool. [@pantoniou](https://github.com/pantoniou) has been exceptionally generous in reviewing and merging patches to support `ghafmt`'s use case.
- [**fyaml**](https://github.com/0k/fyaml) by [@0k](https://github.com/0k) — the Rust bindings to libfyaml that make it possible to use from this codebase.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of [MIT](LICENSE) or [Apache-2.0](LICENSE-APACHE) at your option.
