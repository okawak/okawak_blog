#!/usr/bin/env bash
set -euo pipefail

SECRET_NAME="blog/iam_access_key" # Terraform で作成した Secret 名
SRC_PROFILE="secret-get"
DST_PROFILE="blog-s3" # Leptos が使うプロファイル
if [ -n "$XDG_CONFIG_HOME" ]; then
  AWS_CONFIG_DIR="$XDG_CONFIG_HOME/aws"
else
  AWS_CONFIG_DIR="$HOME/.aws"
fi
CRED_FILE="$AWS_CONFIG_DIR/credentials"

# 1. 最新キーを取得
json=$(AWS_PROFILE=$SRC_PROFILE aws secretsmanager get-secret-value \
  --secret-id "$SECRET_NAME" --query SecretString --output text)

new_id=$(echo "$json" | jq -r .aws_access_key_id)
new_secret=$(echo "$json" | jq -r .aws_secret_access_key)

# 2. credentials を置き換え（DST_PROFILE 部分だけ書き直す）
tmp=$(mktemp)
awk -v p="[$DST_PROFILE]" '
  BEGIN {skip=0}
  $0==p {print; getline; getline; skip=1; next}
  skip && NF==0 {skip=0; next}
  !skip {print}
' "$CRED_FILE" 2>/dev/null || true >"$tmp"

cat >>"$tmp" <<EOF
[$DST_PROFILE]
aws_access_key_id     = $new_id
aws_secret_access_key = $new_secret
region                = ap-northeast-1
EOF

install -m 600 -o "$USER" -g "$USER" "$tmp" "$CRED_FILE"
rm -f "$tmp"

# 3. Leptos サービスを再起動
sudo systemctl restart okawak_blog

echo "$(date '+%F %T') credentials updated to $new_id" \
  >>"$HOME/creds_update.log"
