#!/usr/bin/env bash
set -euo pipefail

SECRET_NAME="blog/iam_access_key" # Terraform で作成した Secret 名
SRC_PROFILE="secret-get"
DST_PROFILE="blog-s3" # Leptos が使うプロファイル
RUNTIME_CREDENTIAL_FILE="${OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE:-/var/lib/okawak_blog/aws/credentials}"
RUNTIME_CREDENTIAL_DIR=$(dirname "$RUNTIME_CREDENTIAL_FILE")
SERVICE_USER=$(id -un)
SERVICE_GROUP=$(id -gn)

if [ "$(id -u)" -eq 0 ]; then
  echo "Run this script as the okawak service user, not root." >&2
  exit 1
fi

umask 077

prepare_runtime_credential_dir() {
  if [ -L "$RUNTIME_CREDENTIAL_DIR" ]; then
    echo "Runtime credential directory must not be a symbolic link: $RUNTIME_CREDENTIAL_DIR" >&2
    exit 1
  fi

  if [ -e "$RUNTIME_CREDENTIAL_DIR" ]; then
    if [ ! -d "$RUNTIME_CREDENTIAL_DIR" ]; then
      echo "Runtime credential directory path is not a directory: $RUNTIME_CREDENTIAL_DIR" >&2
      exit 1
    fi
    if [ ! -O "$RUNTIME_CREDENTIAL_DIR" ] || [ ! -w "$RUNTIME_CREDENTIAL_DIR" ]; then
      echo "Existing runtime credential directory must be owned and writable by $SERVICE_USER: $RUNTIME_CREDENTIAL_DIR" >&2
      exit 1
    fi
    return
  fi

  if ! install -d -m 700 "$RUNTIME_CREDENTIAL_DIR" 2>/dev/null; then
    sudo install -d -m 700 -o "$SERVICE_USER" -g "$SERVICE_GROUP" "$RUNTIME_CREDENTIAL_DIR"
  fi
}

# 1. 既存directoryの権限は変更せず、未作成のruntime専用directoryだけを作る
prepare_runtime_credential_dir

# 2. 最新キーを取得
json=$(AWS_PROFILE=$SRC_PROFILE aws secretsmanager get-secret-value \
  --secret-id "$SECRET_NAME" --query SecretString --output text)

new_id=$(printf '%s' "$json" | jq -er '.aws_access_key_id | select(type == "string" and length > 0)')
new_secret=$(printf '%s' "$json" | jq -er '.aws_secret_access_key | select(type == "string" and length > 0)')

# 3. runtime 専用 credentials file を同一 directory 内で atomic に置き換える
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

# 4. 通常のrotationではLeptos serviceを再起動する。初回移行ではdeploy前に省略できる。
if [ "${OKAWAK_BLOG_SKIP_RESTART:-0}" != "1" ]; then
  sudo systemctl restart okawak_blog
fi

echo "$(date '+%F %T') runtime credentials updated"
