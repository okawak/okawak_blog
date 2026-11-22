#!/usr/bin/env bash

set -euo pipefail

fail() {
  echo "versions-check: $*" >&2
  exit 1
}

mise_bun_version="$(sed -nE 's/^bun = "([^"]+)"$/\1/p' mise.toml)"
topcoat_cli_version="$(sed -nE 's/^"cargo:topcoat-cli" = "([^"]+)"$/\1/p' mise.toml)"
topcoat_framework_version="$(sed -nE 's/^topcoat = \{ version = "=([^"]+)".*/\1/p' Cargo.toml)"

for version in "$mise_bun_version" "$topcoat_cli_version" "$topcoat_framework_version"; do
  [ -n "$version" ] || fail "required version is missing from project configuration"
done

[ "$topcoat_cli_version" = "$topcoat_framework_version" ] \
  || fail "Topcoat CLI $topcoat_cli_version does not match framework $topcoat_framework_version"
mise lock --check --platform linux-x64,linux-arm64,macos-x64,macos-arm64,windows-x64 aqua:tailwindlabs/tailwindcss
[ "$(bun --version)" = "$mise_bun_version" ] \
  || fail "active Bun $(bun --version) does not match mise $mise_bun_version"
[ "$(topcoat --version | awk '{print $2}')" = "$topcoat_cli_version" ] \
  || fail "active Topcoat CLI $(topcoat --version | awk '{print $2}') does not match mise $topcoat_cli_version"

if grep -R -n -E \
  'BUN_VERSION|TOPCOAT_CLI_VERSION|TOPCOAT_TAILWIND_VERSION|oven-sh/setup-bun|topcoat-cli-installer' \
  .github/workflows; then
  fail "workflow-local tool version or installer found"
fi

echo "versions-check: shared tool versions are consistent"
