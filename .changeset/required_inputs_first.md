---
default: minor
---

Sort required inputs and secrets first

`workflow_call` inputs and secrets, `workflow_dispatch` inputs and action
`inputs` are now sorted with `required: true` entries first, followed by the
rest, each group in alphabetical order.

Fixes #217
