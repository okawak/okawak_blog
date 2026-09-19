#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
validation_tmp="$(mktemp -d)"
trap 'rm -rf "$validation_tmp"' EXIT
cp -R "$repo_root/e2e/fixtures/site" "$validation_tmp/site"
bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/site"
rm "$validation_tmp/site/en/articles/tech/e2e-article.html"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/site" >/dev/null 2>&1; then
  echo "Missing English article must stop publication" >&2
  exit 1
fi
cp "$repo_root/e2e/fixtures/site/en/articles/tech/e2e-article.html" "$validation_tmp/site/en/articles/tech/"
printf '%s\n' '{"total_articles":99}' > "$validation_tmp/site/en/metadata/site.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/site" >/dev/null 2>&1; then
  echo "Inconsistent English count must stop publication" >&2
  exit 1
fi
echo "Public artifact gate tests passed"
