---
default: minor
---

Stop renaming workflow interface identifiers

`workflow_call` inputs, secrets and outputs, and `workflow_dispatch` inputs, are
no longer converted to `snake_case`. They are a workflow's public interface and
are referenced from files `ghafmt` never sees (callers' `with:`/`secrets:`, `gh
workflow run -f`, the REST API), so renaming them silently broke callers. Job
IDs, step IDs and job-level outputs are still converted, with references updated.

Fixes #216
