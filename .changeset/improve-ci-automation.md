---
default: internal
---

Improve Renovate automation and migrate release workflow off PAT

- Disable patch-level updates for `taiki-e/install-action` to reduce PR noise.
- `dependency_changeset` workflow now mints a GitHub App token and amends the
  changeset directly into Renovate's commit (force-push with lease), so PRs
  have a single commit. This avoids stale-approval dismissal under the
  `require_last_push_approval` ruleset and triggers required status checks
  on the final SHA (App tokens trigger downstream workflows; `GITHUB_TOKEN`
  pushes do not).
- `release` workflow swaps `RELEASE_PAT` for the same GitHub App token when
  invoking `knope release`, removing the long-lived personal access token.

Requires repo secrets `AUTOMATION_APP_ID` and `AUTOMATION_APP_PRIVATE_KEY`,
and the App to be added as a bypass actor on the "Release tags" ruleset.