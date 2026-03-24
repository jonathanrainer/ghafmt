---
default: patch
---

# Fix action mode flag and remove redundant version input

Fix the `--mode` flag construction in the composite action so users can pass
`mode: check` rather than `mode: mode=check`.

Remove the `version` input and derive the binary version directly from
`github.action_ref`, keeping the action version and binary version in lockstep.