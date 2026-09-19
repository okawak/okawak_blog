#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
validation_tmp="$(mktemp -d)"
trap 'rm -rf "$validation_tmp"' EXIT
# The browser fixture includes malformed Physics data for its 500-response test.
# Publication fixtures must contain only valid, advertised category artifacts.
cp -R "$repo_root/e2e/fixtures/site" "$validation_tmp/valid"
rm "$validation_tmp/valid/categories/physics.json"
cp -R "$validation_tmp/valid" "$validation_tmp/site"
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
# A locale advertised by an article must also have a site snapshot at home.
cp -R "$validation_tmp/valid" "$validation_tmp/missing-locale"
jq '.routes["/"] = ["ja"]' "$validation_tmp/missing-locale/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/missing-locale/locales.json"
rm -r "$validation_tmp/missing-locale/en"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/missing-locale" >/dev/null 2>&1; then
  echo "A locale outside the home snapshot must stop publication" >&2
  exit 1
fi
# Japanese-only releases remain valid when every advertised route is Japanese.
jq '.routes |= with_entries(.value = ["ja"])' "$validation_tmp/missing-locale/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/missing-locale/locales.json"
bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/missing-locale"
cp -R "$validation_tmp/valid" "$validation_tmp/undeclared-article"
jq '.routes["/tech/e2e-article"] = ["ja"]' "$validation_tmp/undeclared-article/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/undeclared-article/locales.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/undeclared-article" >/dev/null 2>&1; then
  echo "Every indexed article must declare its locale" >&2
  exit 1
fi
cp -R "$validation_tmp/valid" "$validation_tmp/unindexed-article"
jq '.articles = []' "$validation_tmp/unindexed-article/en/articles/index.json" > "$validation_tmp/index.json"
mv "$validation_tmp/index.json" "$validation_tmp/unindexed-article/en/articles/index.json"
jq '.total_articles = 0' "$validation_tmp/unindexed-article/en/metadata/site.json" > "$validation_tmp/metadata.json"
mv "$validation_tmp/metadata.json" "$validation_tmp/unindexed-article/en/metadata/site.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/unindexed-article" >/dev/null 2>&1; then
  echo "Every declared article must appear in the locale index" >&2
  exit 1
fi
for about_mode in missing english-only; do
  cp -R "$validation_tmp/valid" "$validation_tmp/about-$about_mode"
  jq --arg mode "$about_mode" 'if $mode == "missing" then del(.routes["/about"]) else .routes["/about"] = ["en"] end' \
    "$validation_tmp/about-$about_mode/locales.json" > "$validation_tmp/locales.json"
  mv "$validation_tmp/locales.json" "$validation_tmp/about-$about_mode/locales.json"
  rm "$validation_tmp/about-$about_mode/pages/about.json"
  if [ "$about_mode" = missing ]; then rm "$validation_tmp/about-$about_mode/en/pages/about.json"; fi
  if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/about-$about_mode" >/dev/null 2>&1; then
    echo "Japanese About is mandatory even when its route is not declared: $about_mode" >&2
    exit 1
  fi
done
cp -R "$validation_tmp/valid" "$validation_tmp/japanese-about"
jq '.routes["/about"] = ["ja"]' "$validation_tmp/japanese-about/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/japanese-about/locales.json"
rm "$validation_tmp/japanese-about/en/pages/about.json"
bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/japanese-about"
for category_mode in missing wrong-count extra duplicate missing-route; do
  cp -R "$validation_tmp/valid" "$validation_tmp/category-$category_mode"
  jq --arg mode "$category_mode" '
    if $mode == "missing" then .categories = []
    elif $mode == "wrong-count" then .categories[0].article_count = 99
    elif $mode == "extra" then .categories += [{"category":"extra","article_count":0}]
    elif $mode == "duplicate" then .categories += [.categories[0]]
    else .categories = [] end
  ' "$validation_tmp/category-$category_mode/en/metadata/site.json" > "$validation_tmp/metadata.json"
  mv "$validation_tmp/metadata.json" "$validation_tmp/category-$category_mode/en/metadata/site.json"
  if [ "$category_mode" = missing-route ]; then
    jq '.routes["/tech"] = ["ja"]' "$validation_tmp/category-$category_mode/locales.json" > "$validation_tmp/locales.json"
    mv "$validation_tmp/locales.json" "$validation_tmp/category-$category_mode/locales.json"
  fi
  if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/category-$category_mode" >/dev/null 2>&1; then
    echo "Category metadata must match routes and article counts: $category_mode" >&2
    exit 1
  fi
done
cp -R "$validation_tmp/valid" "$validation_tmp/extra-category-article"
jq '.articles += [.articles[0]]' "$validation_tmp/extra-category-article/en/categories/tech.json" > "$validation_tmp/category.json"
mv "$validation_tmp/category.json" "$validation_tmp/extra-category-article/en/categories/tech.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/extra-category-article" >/dev/null 2>&1; then
  echo "Category articles must match the locale index exactly" >&2
  exit 1
fi
# A published landing with no articles still belongs in category metadata.
cp -R "$validation_tmp/valid" "$validation_tmp/empty-category"
jq '.routes["/daily"] = ["ja"]' "$validation_tmp/empty-category/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/empty-category/locales.json"
jq '.categories += [{"category":"daily","article_count":0}]' "$validation_tmp/empty-category/metadata/site.json" > "$validation_tmp/metadata.json"
mv "$validation_tmp/metadata.json" "$validation_tmp/empty-category/metadata/site.json"
jq '.category = "daily" | .articles = []' "$validation_tmp/empty-category/categories/tech.json" > "$validation_tmp/empty-category/categories/daily.json"
bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/empty-category"
jq 'del(.routes["/daily"])' "$validation_tmp/empty-category/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/empty-category/locales.json"
jq '.categories |= map(select(.category != "daily"))' "$validation_tmp/empty-category/metadata/site.json" > "$validation_tmp/metadata.json"
mv "$validation_tmp/metadata.json" "$validation_tmp/empty-category/metadata/site.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/empty-category" >/dev/null 2>&1; then
  echo "A category artifact must remain declared even when it has no articles" >&2
  exit 1
fi
echo "Public artifact gate tests passed"
