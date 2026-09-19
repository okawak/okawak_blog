#!/usr/bin/env bash
# Validate a completed local release before upload; no vault or AI dependency.
set -euo pipefail
root="${1:?Usage: validate_public_artifacts.sh SITE_ROOT}"
jq -e '
  .schema_version == 1 and (.routes | type == "object") and
  (.routes["/"] | index("ja") != null) and
  ([.routes[][] | . == "ja" or . == "en"] | all)
' "$root/locales.json" >/dev/null

for locale in $(jq -r '.routes["/"][]' "$root/locales.json"); do
  locale_root="$root"
  if [ "$locale" = en ]; then locale_root="$root/en"; fi
  jq -e '.articles | type == "array"' "$locale_root/articles/index.json" >/dev/null
  jq -e 'type == "object" and ([.[] | type == "string"] | all)' "$locale_root/tags.json" >/dev/null
  article_count="$(jq '.articles | length' "$locale_root/articles/index.json")"
  jq -e --argjson count "$article_count" '.total_articles == $count' "$locale_root/metadata/site.json" >/dev/null
  while IFS=$'\t' read -r category slug; do
    test -s "$locale_root/articles/$category/$slug.html"
    jq -e --arg slug "$slug" 'any(.articles[]; .slug == $slug)' "$locale_root/categories/$category.json" >/dev/null
  done < <(jq -r '.articles[] | [.category, .slug] | @tsv' "$locale_root/articles/index.json")
  while IFS= read -r route; do
    case "$route" in
      /) ;;
      /about) test -s "$locale_root/pages/about.json" ;;
      /*/*) test -s "$locale_root/articles${route}.html" ;;
      /*) test -s "$locale_root/categories${route}.json" ;;
    esac
  done < <(jq -r --arg locale "$locale" '.routes | to_entries[] | select(.value | index($locale)) | .key' "$root/locales.json")
  echo "Validated $locale: $article_count articles"
done
