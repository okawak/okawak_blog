#!/usr/bin/env bash

set -euo pipefail

fail() {
  echo "versions-check: $*" >&2
  exit 1
}

mise_bun_version="$(sed -nE 's/^bun = "([^"]+)"$/\1/p' mise.toml)"
cargo_leptos_version="$(sed -nE 's/^"github:leptos-rs\/cargo-leptos" = "([^"]+)"$/\1/p' mise.toml)"
leptosfmt_version="$(sed -nE 's/^"github:bram209\/leptosfmt" = "([^"]+)"$/\1/p' mise.toml)"
tailwind_version="$(sed -nE 's/^LEPTOS_TAILWIND_VERSION = "v([^"]+)"$/\1/p' mise.toml)"
e2e_bun_version="$(sed -nE 's/.*"packageManager": "bun@([^"]+)".*/\1/p' e2e/package.json)"
tailwind_cli_version="$(sed -nE 's/.*"@tailwindcss\/cli": "([^"]+)".*/\1/p' crates/site/web/package.json)"
tailwind_package_version="$(sed -nE 's/.*"tailwindcss": "([^"]+)".*/\1/p' crates/site/web/package.json)"

for version in "$mise_bun_version" "$cargo_leptos_version" "$leptosfmt_version" "$tailwind_version"; do
  [ -n "$version" ] || fail "required version is missing from mise.toml"
done

[ "$mise_bun_version" = "$e2e_bun_version" ] \
  || fail "mise Bun $mise_bun_version does not match e2e packageManager $e2e_bun_version"
[ "$tailwind_version" = "$tailwind_cli_version" ] \
  || fail "Tailwind CLI $tailwind_cli_version does not match LEPTOS_TAILWIND_VERSION $tailwind_version"
[ "$tailwind_version" = "$tailwind_package_version" ] \
  || fail "Tailwind package $tailwind_package_version does not match LEPTOS_TAILWIND_VERSION $tailwind_version"
[ "$(bun --version)" = "$mise_bun_version" ] \
  || fail "active Bun $(bun --version) does not match mise $mise_bun_version"
[ "$(cargo leptos --version | awk '{print $2}')" = "$cargo_leptos_version" ] \
  || fail "active cargo-leptos does not match mise $cargo_leptos_version"
[ "$(leptosfmt --version | awk '{print $2}')" = "$leptosfmt_version" ] \
  || fail "active leptosfmt does not match mise $leptosfmt_version"

if grep -R -n -E \
  'BUN_VERSION|CARGO_LEPTOS_VERSION|LEPTOS_TAILWIND_VERSION|oven-sh/setup-bun|cargo-leptos-installer' \
  .github/workflows; then
  fail "workflow-local tool version or installer found"
fi

for workflow in .github/workflows/ci.yml .github/workflows/upload.yml; do
  grep -q 'jdx/mise-action@v4' "$workflow" \
    || fail "$workflow does not use jdx/mise-action@v4"
done

echo "versions-check: shared tool versions are consistent"
