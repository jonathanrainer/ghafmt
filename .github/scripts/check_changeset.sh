#!/usr/bin/env bash
# Verifies that a changeset file exists on the current branch before committing.
# Passes if:
#   - currently on main/master (no changeset required for direct commits), or
#   - a .changeset/*.md file is staged for this commit, or
#   - a .changeset/*.md file was already added earlier on this branch vs main.
set -euo pipefail

branch=$(git rev-parse --abbrev-ref HEAD)

if [ "$branch" = "main" ] || [ "$branch" = "master" ]; then
  exit 0
fi

staged=$(git diff --name-only --diff-filter=A --cached -- '.changeset/*.md')
committed=$(git diff --name-only --diff-filter=A main...HEAD -- '.changeset/*.md' 2>/dev/null || true)

if [ -z "$staged" ] && [ -z "$committed" ]; then
  echo "error: no changeset file found for this branch."
  echo "Add a .changeset/<descriptive_name>.md before committing."
  echo "See CONTRIBUTING.md for the required format."
  exit 1
fi