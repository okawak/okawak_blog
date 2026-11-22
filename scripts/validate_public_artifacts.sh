#!/usr/bin/env bash
# Validate a completed local release before upload; no vault or AI dependency.
set -euo pipefail
root="${1:?Usage: validate_public_artifacts.sh SITE_ROOT [PUBLISH_BINARY]}"
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
if [ -n "${2:-}" ]; then
  "$2" --validate-artifacts "$root"
else
  cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p blog-publish -- --validate-artifacts "$root"
fi
jq -e '
  .routes["/"] as $home_locales |
  .schema_version == 1 and (.routes | type == "object") and
  (.routes["/"] | index("ja") != null) and
  (.routes["/about"] | index("ja") != null) and
  ([.routes[] | (. - $home_locales | length) == 0] | all)
' "$root/locales.json" >/dev/null
test -s "$root/ja/about.json"

for locale in $(jq -r '.routes["/"][]' "$root/locales.json"); do
  locale_root="$root/$locale"
  jq -e '.articles | type == "array"' "$locale_root/_data/articles.json" >/dev/null
  jq -e 'type == "object" and ([.[] | type == "string"] | all)' "$locale_root/_data/tags.json" >/dev/null
  article_count="$(jq '.articles | length' "$locale_root/_data/articles.json")"
  jq -e --arg locale "$locale" --slurpfile routes "$root/locales.json" \
    --slurpfile index "$locale_root/_data/articles.json" '
    [$routes[0].routes | to_entries[] | select(.value | index($locale)) | .key |
      select(test("^/[^/]+$") and . != "/about") | ltrimstr("/")] as $categories |
    .total_articles == ($index[0].articles | length) and
    ([.categories[].article_count] | add // 0) == .total_articles and
    (.categories | sort_by(.category)) ==
      ([$categories[] as $category | {category: $category,
        article_count: ([$index[0].articles[] | select(.category == $category)] | length)}] |
       sort_by(.category))
  ' "$locale_root/_data/site.json" >/dev/null
  # Readers can fetch pages by their keys directly. Every page file must be advertised.
  for page in "$locale_root"/*.json "$locale_root"/*/*.json; do
    [ -f "$page" ] || continue
    relative="${page#"$locale_root"/}"
    case "$relative" in index.json|_data/*) continue ;; esac
    jq -e --arg route "/${relative%.json}" --arg locale "$locale" \
      '.routes[$route] | index($locale) != null' "$root/locales.json" >/dev/null
  done
  while IFS= read -r category; do
    jq -e --arg category "$category" --slurpfile index "$locale_root/_data/articles.json" '
      .category == $category and (.articles | sort_by(.slug)) ==
        ([$index[0].articles[] | select(.category == $category)] | sort_by(.slug))
    ' "$locale_root/$category.json" >/dev/null
  done < <(jq -r '.categories[].category' "$locale_root/_data/site.json")
  while IFS= read -r route; do
    case "$route" in
      /) ;;
      /about) test -s "$locale_root/about.json" ;;
      /*/*)
        test -s "$locale_root${route}.json"
        jq -e --arg route "$route" 'any(.articles[]; "/" + .category + "/" + .slug == $route)' "$locale_root/_data/articles.json" >/dev/null
        ;;
      /*) test -s "$locale_root${route}.json" ;;
    esac
  done < <(jq -r --arg locale "$locale" '.routes | to_entries[] | select(.value | index($locale)) | .key' "$root/locales.json")
  echo "Validated $locale: $article_count articles"
done
