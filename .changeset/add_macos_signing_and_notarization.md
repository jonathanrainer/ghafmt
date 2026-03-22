---
default: minor
---

# macOS binary signing and notarization

Released macOS binaries are now signed with a Developer ID Application certificate and notarized
with Apple, so Gatekeeper no longer blocks them on download.

Signing and notarization is implemented as a composite action
(`.github/actions/sign-and-notarize`) following the existing `set-cargo-version` pattern, and
is called from the build artifacts workflow for macOS targets only. It runs exclusively on `main`
so signing credentials are never exposed during PR or feature branch builds.