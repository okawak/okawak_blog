#!/usr/bin/env bash
set -euo pipefail
root=${1:?site directory required}
manifest=${2:?manifest required}
: "${S3_BUCKET:?}" "${RELEASE_ID:?}" "${ARTIFACT_PREFIX:?}"
[[ "$ARTIFACT_PREFIX" == "releases/$RELEASE_ID/site" && "$RELEASE_ID" != */* ]]
put_new() {
  aws s3api put-object --bucket "$S3_BUCKET" --key "$1" --body "$2" \
    --if-none-match '*' --cache-control 'public, max-age=31536000, immutable' \
    --content-type "$3" >/dev/null
}
find "$root" -type f -print0 | while IFS= read -r -d '' file; do
  case "$file" in
    *.json) mime=application/json ;;
    *.html) mime=text/html ;;
    *.jpg|*.jpeg) mime=image/jpeg ;;
    *.png|*.gif|*.webp|*.avif) mime="image/${file##*.}" ;;
    *) mime=application/octet-stream ;;
  esac
  put_new "$ARTIFACT_PREFIX/${file#"$root"/}" "$file" "$mime"
done
put_new "releases/$RELEASE_ID/manifest.json" "$manifest" application/json
