---
"ghafmt": patch
---

Fix spurious blank lines in block-sequence `needs` lists

The `StepsBlankLines` presenter was retaining state across jobs. After
processing a job's `steps` sequence, the internal state remained as
`Step { is_first: false }`. Any block-sequence `needs` items in the
following job emitted `-` indicators at the same indent depth (6 spaces),
satisfying the step-detection condition and triggering unwanted blank line
insertion before the first item and between subsequent items.

The fix resets state to `Init` whenever any key other than `steps` is seen
at the job-key indent depth, preventing stale step-tracking state from
bleeding across job boundaries.