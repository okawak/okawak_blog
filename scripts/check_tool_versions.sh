#!/usr/bin/env bash

set -euo pipefail

fail() {
  echo "versions-check: $*" >&2
  exit 1
}

mise_bun_version="$(sed -nE 's/^bun = "([^"]+)"$/\1/p' mise.toml)"
topcoat_cli_version="$(sed -nE 's/^"cargo:topcoat-cli" = "([^"]+)"$/\1/p' mise.toml)"
topcoat_framework_version="$(sed -nE 's/^topcoat = \{ version = "=([^"]+)".*/\1/p' Cargo.toml)"
tailwind_version="$(sed -nE 's/^TOPCOAT_TAILWIND_VERSION = "([^"]+)"$/\1/p' mise.toml)"
topcoat_tailwind_version="$(sed -nE 's/^[[:space:]]*\.version\("([^"]+)"\)$/\1/p' crates/server/build.rs)"

for version in "$mise_bun_version" "$topcoat_cli_version" "$topcoat_framework_version" "$tailwind_version" "$topcoat_tailwind_version"; do
  [ -n "$version" ] || fail "required version is missing from project configuration"
done

[ "$topcoat_cli_version" = "$topcoat_framework_version" ] \
  || fail "Topcoat CLI $topcoat_cli_version does not match framework $topcoat_framework_version"
[ "$tailwind_version" = "$topcoat_tailwind_version" ] \
  || fail "Topcoat Tailwind $topcoat_tailwind_version does not match TOPCOAT_TAILWIND_VERSION $tailwind_version"
[ "$(bun --version)" = "$mise_bun_version" ] \
  || fail "active Bun $(bun --version) does not match mise $mise_bun_version"
[ "$(topcoat fmt --version | awk '{print $2}')" = "$topcoat_cli_version" ] \
  || fail "active Topcoat CLI $(topcoat fmt --version | awk '{print $2}') does not match mise $topcoat_cli_version"

if grep -R -n -E \
  'BUN_VERSION|TOPCOAT_CLI_VERSION|TOPCOAT_TAILWIND_VERSION|oven-sh/setup-bun|topcoat-cli-installer' \
  .github/workflows; then
  fail "workflow-local tool version or installer found"
fi

for workflow in .github/workflows/ci.yml .github/workflows/upload.yml; do
  # Renovate pins action refs to commit SHAs while retaining the version in a comment.
  grep -Eq '^[[:space:]]*uses:[[:space:]]+jdx/mise-action@(v[0-9]+|[[:xdigit:]]{40})([[:space:]]|$)' "$workflow" \
    || fail "$workflow does not use jdx/mise-action with a major tag or full commit SHA"
done

echo "versions-check: shared tool versions are consistent"
