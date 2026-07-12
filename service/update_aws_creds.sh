#!/usr/bin/env bash
set -euo pipefail

SECRET_NAME="blog/iam_access_key" # Terraform で作成した Secret 名
SRC_PROFILE="secret-get"
DST_PROFILE="blog-s3" # Leptos が使うプロファイル
RUNTIME_CREDENTIAL_FILE="${OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE:-/var/lib/okawak_blog/aws/credentials}"
RUNTIME_CREDENTIAL_DIR=$(dirname "$RUNTIME_CREDENTIAL_FILE")

umask 077

# 1. 最新キーを取得
json=$(AWS_PROFILE=$SRC_PROFILE aws secretsmanager get-secret-value \
  --secret-id "$SECRET_NAME" --query SecretString --output text)

new_id=$(printf '%s' "$json" | jq -er '.aws_access_key_id | select(type == "string" and length > 0)')
new_secret=$(printf '%s' "$json" | jq -er '.aws_secret_access_key | select(type == "string" and length > 0)')

# 2. runtime 専用 credentials file を同一 directory 内で atomic に置き換える
install -d -m 700 "$RUNTIME_CREDENTIAL_DIR"
tmp=$(mktemp "$RUNTIME_CREDENTIAL_DIR/.credentials.XXXXXX")
trap 'rm -f "$tmp"' EXIT

cat >"$tmp" <<EOF
[$DST_PROFILE]
aws_access_key_id     = $new_id
aws_secret_access_key = $new_secret
EOF

chmod 600 "$tmp"
mv -f "$tmp" "$RUNTIME_CREDENTIAL_FILE"
trap - EXIT

# 3. Leptos サービスを再起動
sudo systemctl restart okawak_blog

echo "$(date '+%F %T') runtime credentials updated"
