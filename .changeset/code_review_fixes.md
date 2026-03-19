---
default: patch
---

# Code review fixes and improvements

Address findings from a Staff Engineer-level review of the codebase.

## Bug fixes
- Fix typo in `FilterSorter`: `on/pull_request_targe/branches-ignore` →
  `on/pull_request_target/branches-ignore`, which was silently skipping that
  sort path
- Remove `outputs` and `secrets` from `WorkflowDispatchSorter`; those keys
  belong to `workflow_call` only and were being incorrectly sorted
- Fix `Rename::Ord`/`PartialOrd`/`Eq`/`PartialEq` impls in `CaseEnforcer` that
  compared only depth, violating the `Eq`/`Ord` contract; sorting is now done
  via `sort_by_key` with `Reverse` on depth
- Reject `stdin (-)` with `--mode=list`, consistent with the existing
  `--mode=write` restriction; produces a structured error via the same
  `StdinCannotBeUsedWithList` variant

## Refactoring
- Replace ad-hoc `eprintln!` calls for early validation errors in `Ghafmt::run`
  with structured `Error` variants rendered through `render_error`, so colour
  mode is respected and diagnostics are consistent
- Extract `build_handler` and `render_error` as `pub(crate)` free functions;
  remove them from the `Command` trait entirely — callers use the free functions
  directly
- Simplify `FormatterResult`: replace `original: Option<String>` + accessor
  method with a plain `pub(crate) original: String` field
- Change `Ghafmt::run` signature from `mut self` to `&mut self`
- Pre-compute static key-ordering `Vec<String>` fields in transformer struct
  constructors instead of allocating on every `process()` call; affected
  transformers: `TopLevelSorter`, `JobSorter`, `StepSorter`, `ConcurrencySorter`,
  `StrategySorter`, `ContainerSorter`, `RunsOnSorter`, `EnvironmentSorter`,
  `WorkflowCallSorter`
- Change `sort_mapping_at_path` and `classify_renames` signatures to accept
  `&[T]` instead of `&Vec<T>`
- Make the `WorkflowProcessor` transformer pipeline injectable via
  `WorkflowProcessor::new(Vec<Box<dyn StructureTransformer>>)` with a
  `Default` impl that builds the standard pipeline; removes the need for a
  test-only constructor
- Derive `Copy` on `State` enums in `JobsBlankLines` and `StepsBlankLines`;
  remove now-redundant `.clone()` calls at match sites
- Remove stale `#[allow(unused_assignments)]` from `errors.rs`
- Fix `--colour` flag spelling in the CLI integration test (was `--color`,
  causing the test to pass vacuously against a clap parse error)

## Tests
- Add unit tests for `source_window` and `line_col_to_byte_offset` in `errors.rs`
- Add integration tests for `WorkflowProcessor` error recovery: failed
  transformer emits a warning, document is restored, and subsequent transformers
  still run
- Add `list_mode_rejects_stdin` CLI integration test
- Add `pull_request_target_branches_ignore_sorted` regression test in `FilterSorter`
- Remove duplicate `multi_job_comments_between_steps` test case in `StepsBlankLines`
