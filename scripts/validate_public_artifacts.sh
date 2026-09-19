#!/usr/bin/env bash
# Validate a completed local release before upload; no vault or AI dependency.
set -euo pipefail
root="${1:?Usage: validate_public_artifacts.sh SITE_ROOT}"
jq -e '
  .routes["/"] as $home_locales |
  .schema_version == 1 and (.routes | type == "object") and
  (.routes["/"] | index("ja") != null) and
  (.routes["/about"] | index("ja") != null) and
  ([.routes[][] | . == "ja" or . == "en"] | all) and
  ([.routes[] | (. - $home_locales | length) == 0] | all)
' "$root/locales.json" >/dev/null
test -s "$root/pages/about.json"

for locale in $(jq -r '.routes["/"][]' "$root/locales.json"); do
  locale_root="$root"
  if [ "$locale" = en ]; then locale_root="$root/en"; fi
  jq -e '.articles | type == "array"' "$locale_root/articles/index.json" >/dev/null
  jq -e 'type == "object" and ([.[] | type == "string"] | all)' "$locale_root/tags.json" >/dev/null
  article_count="$(jq '.articles | length' "$locale_root/articles/index.json")"
  jq -e --arg locale "$locale" --slurpfile routes "$root/locales.json" \
    --slurpfile index "$locale_root/articles/index.json" '
    [$routes[0].routes | to_entries[] | select(.value | index($locale)) | .key |
      select(test("^/[^/]+$") and . != "/about") | ltrimstr("/")] as $categories |
    .total_articles == ($index[0].articles | length) and
    ([.categories[].article_count] | add // 0) == .total_articles and
    (.categories | sort_by(.category)) ==
      ([$categories[] as $category | {category: $category,
        article_count: ([$index[0].articles[] | select(.category == $category)] | length)}] |
       sort_by(.category))
  ' "$locale_root/metadata/site.json" >/dev/null
  while IFS= read -r category; do
    jq -e --arg category "$category" --slurpfile index "$locale_root/articles/index.json" '
      .category == $category and (.articles | sort_by(.slug)) ==
        ([$index[0].articles[] | select(.category == $category)] | sort_by(.slug))
    ' "$locale_root/categories/$category.json" >/dev/null
  done < <(jq -r '.categories[].category' "$locale_root/metadata/site.json")
  while IFS=$'\t' read -r category slug; do
    jq -e --arg route "/$category/$slug" --arg locale "$locale" '.routes[$route] | index($locale) != null' "$root/locales.json" >/dev/null
    test -s "$locale_root/articles/$category/$slug.html"
  done < <(jq -r '.articles[] | [.category, .slug] | @tsv' "$locale_root/articles/index.json")
  while IFS= read -r route; do
    case "$route" in
      /) ;;
      /about) test -s "$locale_root/pages/about.json" ;;
      /*/*)
        test -s "$locale_root/articles${route}.html"
        jq -e --arg route "$route" 'any(.articles[]; "/" + .category + "/" + .slug == $route)' "$locale_root/articles/index.json" >/dev/null
        ;;
      /*) test -s "$locale_root/categories${route}.json" ;;
    esac
  done < <(jq -r --arg locale "$locale" '.routes | to_entries[] | select(.value | index($locale)) | .key' "$root/locales.json")
  echo "Validated $locale: $article_count articles"
done
