---
default: minor
---

# Refactor: split `main.rs`, improve CLI, harden stdin reading

#### Breaking changes
- The `--check`, `--write`, and `--list` flags have been replaced with a single `--mode=<format|check|write|list>` argument. `format` (stdout) is the default.

#### Structural changes
- `main.rs` has been split into focused modules:
  - `src/lib.rs` — orchestration and `FormatterResult`
  - `src/cli.rs` — Clap argument definitions
  - `src/commands/` — one file per mode (`check`, `format`, `write`, `list`), sharing a `Command` trait
  - `src/fs.rs` — `expand_paths`, `atomic_write`, stdin reading

#### Bug fixes
- `read_from_stdin` previously used `take(LIMIT)` to cap reads but then checked `n > LIMIT`, a guard that could never fire. The function now probes for an overflow byte after reading, correctly detecting inputs that exceed the limit while accepting files of exactly the limit size.

#### Dependencies
- Added `atomic-write-file` — replaces the hand-rolled temp-file-then-rename implementation of `atomic_write`.
- Added `patharg` — typed `InputArg` enum wrapping file paths and stdin (`-`).
- Added `strum_macros` — derive-based enum string conversions for CLI mode/colour variants.

#### Tests
- Added unit tests for `read_with_limit` (under limit, exactly at limit, over limit) and `atomic_write` (overwrites rather than appends).
