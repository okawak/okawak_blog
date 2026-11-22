#!/usr/bin/env bash
# Check the effective policy definition and the publisher's request contract offline.
set -euo pipefail
hcl2json terraform/aws/s3/bucket.tf | jq -e '
  .resource.aws_s3_bucket_policy.immutable_releases[0] as $attachment |
  $attachment.bucket == "${aws_s3_bucket.myblog_bucket.id}" and
  $attachment.policy == "${data.aws_iam_policy_document.immutable_releases.json}" and
  any(.data.aws_iam_policy_document.immutable_releases[].statement[];
    .effect == "Deny" and
    .actions == ["s3:PutObject"] and
    .resources == ["${aws_s3_bucket.myblog_bucket.arn}/releases/*"] and
    .principals == [{"type":"*", "identifiers":["*"]}] and
    (.condition | sort_by(.test)) == [
      {"test":"Bool","variable":"s3:ObjectCreationOperation","values":["true"]},
      {"test":"Null","variable":"s3:if-none-match","values":["true"]}
    ])
' >/dev/null

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin" "$tmp/site/articles"
echo '{}' > "$tmp/site/articles/a.json"
echo '{}' > "$tmp/manifest.json"
export PATH="$tmp/bin:$PATH" STORAGE="$tmp/storage" RELEASE_ID=test-release S3_BUCKET=fixture
export ARTIFACT_PREFIX="releases/$RELEASE_ID/site"

cat > "$tmp/bin/aws" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1 $2" == "s3api put-object" ]]
shift 2
key= body= match= bucket=
while (( $# )); do
  case "$1" in
    --key) key=$2 ;;
    --body) body=$2 ;;
    --if-none-match) match=$2 ;;
    --bucket) bucket=$2 ;;
    --content-type|--cache-control) ;;
    *) exit 1 ;;
  esac
  shift 2
done
[[ "$match" == '*' && "$bucket" == fixture && "$key" == releases/test-release/* ]]
mkdir -p "$(dirname "$STORAGE/$key")"
if [[ -e "$STORAGE/$key" ]]; then exit 42; fi
cp "$body" "$STORAGE/$key"
STUB
chmod +x "$tmp/bin/aws"
bash scripts/upload_immutable_release.sh "$tmp/site" "$tmp/manifest.json"
cmp "$tmp/site/articles/a.json" "$STORAGE/$ARTIFACT_PREFIX/articles/a.json"
manifest="$STORAGE/releases/$RELEASE_ID/manifest.json"
cmp "$tmp/manifest.json" "$manifest"
rm "$manifest"
echo 'changed' > "$tmp/site/articles/a.json"
if bash scripts/upload_immutable_release.sh "$tmp/site" "$tmp/manifest.json"; then
  echo 'Publication unexpectedly succeeded on a duplicate release key' >&2
  exit 1
fi
[[ ! -e "$manifest" ]]
[[ "$(cat "$STORAGE/$ARTIFACT_PREFIX/articles/a.json")" == '{}' ]]
echo 'Immutable release policy and upload checks passed'
