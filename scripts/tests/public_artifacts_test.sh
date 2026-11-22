#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
validation_tmp="$(mktemp -d)"
trap 'rm -rf "$validation_tmp"' EXIT
# The browser fixture includes malformed Physics data for its 500-response test.
# Publication fixtures must contain only valid, advertised category artifacts.
cp -R "$repo_root/e2e/fixtures/site" "$validation_tmp/valid"
rm "$validation_tmp/valid/ja/physics.json"
cp -R "$validation_tmp/valid" "$validation_tmp/site"
bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/site"
# Both locales use article JSON.
for article_mode in missing malformed mismatch; do
  cp -R "$validation_tmp/valid" "$validation_tmp/article-$article_mode"
  article_path="$validation_tmp/article-$article_mode/ja/tech/e2e-article.json"
  case "$article_mode" in
    missing)
      # An HTML file cannot replace the required article JSON.
      jq -r '.html' "$article_path" > "${article_path%.json}.html"
      rm "$article_path"
      ;;
    malformed) printf '%s\n' 'invalid json' > "$article_path" ;;
    mismatch)
      jq '.summary.title = "Different title"' "$article_path" > "$validation_tmp/article.json"
      mv "$validation_tmp/article.json" "$article_path"
      ;;
  esac
  if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/article-$article_mode" >/dev/null 2>&1; then
    echo "Invalid article JSON must stop publication: $article_mode" >&2
    exit 1
  fi
done
rm "$validation_tmp/site/en/tech/e2e-article.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/site" >/dev/null 2>&1; then
  echo "Missing English article must stop publication" >&2
  exit 1
fi
cp "$repo_root/e2e/fixtures/site/en/tech/e2e-article.json" "$validation_tmp/site/en/tech/"
printf '%s\n' '{"total_articles":99}' > "$validation_tmp/site/en/_data/site.json"
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
jq '.articles = []' "$validation_tmp/unindexed-article/en/_data/articles.json" > "$validation_tmp/index.json"
mv "$validation_tmp/index.json" "$validation_tmp/unindexed-article/en/_data/articles.json"
jq '.total_articles = 0' "$validation_tmp/unindexed-article/en/_data/site.json" > "$validation_tmp/metadata.json"
mv "$validation_tmp/metadata.json" "$validation_tmp/unindexed-article/en/_data/site.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/unindexed-article" >/dev/null 2>&1; then
  echo "Every declared article must appear in the locale index" >&2
  exit 1
fi
# Leftover JSON is directly readable even if both route and article indexes omit it.
for page in tech/unlisted about; do
  cp -R "$validation_tmp/valid" "$validation_tmp/orphan"
  if [ "$page" = about ]; then
    jq '.routes["/about"] = ["ja"]' "$validation_tmp/orphan/locales.json" > "$validation_tmp/locales.json"
    mv "$validation_tmp/locales.json" "$validation_tmp/orphan/locales.json"
  else
    cp "$validation_tmp/orphan/en/tech/e2e-article.json" "$validation_tmp/orphan/en/$page.json"
  fi
  if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/orphan" >/dev/null 2>&1; then
    echo "Unlisted page JSON must stop publication: $page" >&2
    exit 1
  fi
  rm -r "$validation_tmp/orphan"
done
for about_mode in missing english-only; do
  cp -R "$validation_tmp/valid" "$validation_tmp/about-$about_mode"
  jq --arg mode "$about_mode" 'if $mode == "missing" then del(.routes["/about"]) else .routes["/about"] = ["en"] end' \
    "$validation_tmp/about-$about_mode/locales.json" > "$validation_tmp/locales.json"
  mv "$validation_tmp/locales.json" "$validation_tmp/about-$about_mode/locales.json"
  rm "$validation_tmp/about-$about_mode/ja/about.json"
  if [ "$about_mode" = missing ]; then rm "$validation_tmp/about-$about_mode/en/about.json"; fi
  if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/about-$about_mode" >/dev/null 2>&1; then
    echo "Japanese About is mandatory even when its route is not declared: $about_mode" >&2
    exit 1
  fi
done
cp -R "$validation_tmp/valid" "$validation_tmp/japanese-about"
jq '.routes["/about"] = ["ja"]' "$validation_tmp/japanese-about/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/japanese-about/locales.json"
rm "$validation_tmp/japanese-about/en/about.json"
bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/japanese-about"
for category_mode in missing wrong-count extra duplicate missing-route; do
  cp -R "$validation_tmp/valid" "$validation_tmp/category-$category_mode"
  jq --arg mode "$category_mode" '
    if $mode == "missing" then .categories = []
    elif $mode == "wrong-count" then .categories[0].article_count = 99
    elif $mode == "extra" then .categories += [{"category":"extra","article_count":0}]
    elif $mode == "duplicate" then .categories += [.categories[0]]
    else .categories = [] end
  ' "$validation_tmp/category-$category_mode/en/_data/site.json" > "$validation_tmp/metadata.json"
  mv "$validation_tmp/metadata.json" "$validation_tmp/category-$category_mode/en/_data/site.json"
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
jq '.articles += [.articles[0]]' "$validation_tmp/extra-category-article/en/tech.json" > "$validation_tmp/category.json"
mv "$validation_tmp/category.json" "$validation_tmp/extra-category-article/en/tech.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/extra-category-article" >/dev/null 2>&1; then
  echo "Category articles must match the locale index exactly" >&2
  exit 1
fi
# A published landing with no articles still belongs in category metadata.
cp -R "$validation_tmp/valid" "$validation_tmp/empty-category"
jq '.routes["/daily"] = ["ja"]' "$validation_tmp/empty-category/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/empty-category/locales.json"
jq '.categories += [{"category":"daily","article_count":0}]' "$validation_tmp/empty-category/ja/_data/site.json" > "$validation_tmp/metadata.json"
mv "$validation_tmp/metadata.json" "$validation_tmp/empty-category/ja/_data/site.json"
jq '.category = "daily" | .articles = []' "$validation_tmp/empty-category/ja/tech.json" > "$validation_tmp/empty-category/ja/daily.json"
bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/empty-category"
jq 'del(.routes["/daily"])' "$validation_tmp/empty-category/locales.json" > "$validation_tmp/locales.json"
mv "$validation_tmp/locales.json" "$validation_tmp/empty-category/locales.json"
jq '.categories |= map(select(.category != "daily"))' "$validation_tmp/empty-category/ja/_data/site.json" > "$validation_tmp/metadata.json"
mv "$validation_tmp/metadata.json" "$validation_tmp/empty-category/ja/_data/site.json"
if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/empty-category" >/dev/null 2>&1; then
  echo "A category artifact must remain declared even when it has no articles" >&2
  exit 1
fi
for landing_field in title html updated_at; do
  cp -R "$validation_tmp/valid" "$validation_tmp/invalid-$landing_field"
  jq --arg field "$landing_field" '.[$field] = ""' "$validation_tmp/invalid-$landing_field/en/tech.json" > "$validation_tmp/category.json"
  mv "$validation_tmp/category.json" "$validation_tmp/invalid-$landing_field/en/tech.json"
  if bash "$repo_root/scripts/validate_public_artifacts.sh" "$validation_tmp/invalid-$landing_field" >/dev/null 2>&1; then
    echo "Every category landing must satisfy domain rules: $landing_field" >&2
    exit 1
  fi
done
echo "Public artifact gate tests passed"
