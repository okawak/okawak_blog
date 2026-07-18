#!/usr/bin/env bash
set -euo pipefail

SECRET_NAME="blog/iam_access_key" # Terraform で作成した Secret 名
SRC_PROFILE="secret-get"
DST_PROFILE="blog-s3" # Leptos が使うプロファイル
RUNTIME_CREDENTIAL_FILE="${OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE:-/var/lib/okawak_blog/aws/credentials}"
RUNTIME_CREDENTIAL_DIR=$(dirname "$RUNTIME_CREDENTIAL_FILE")
SERVICE_NAME="${OKAWAK_BLOG_SERVICE_NAME:-okawak_blog}"
READINESS_URL="${OKAWAK_BLOG_READINESS_URL:-http://127.0.0.1:8008/api/ready}"
READINESS_RETRIES="${OKAWAK_BLOG_READINESS_RETRIES:-10}"
READINESS_RETRY_INTERVAL_SECONDS="${OKAWAK_BLOG_READINESS_RETRY_INTERVAL_SECONDS:-1}"
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

check_readiness() {
  local attempt

  for ((attempt = 1; attempt <= READINESS_RETRIES; attempt++)); do
    if curl --fail --silent --show-error --output /dev/null "$READINESS_URL"; then
      return 0
    fi
    if [ "$attempt" -lt "$READINESS_RETRIES" ]; then
      sleep "$READINESS_RETRY_INTERVAL_SECONDS"
    fi
  done

  echo "Readiness check failed: $READINESS_URL" >&2
  return 1
}

rollback_runtime_credential() {
  local previous_file=$1

  if [ -n "$previous_file" ]; then
    mv -f "$previous_file" "$RUNTIME_CREDENTIAL_FILE"
  else
    rm -f "$RUNTIME_CREDENTIAL_FILE"
  fi

  if sudo -n systemctl restart "$SERVICE_NAME" && check_readiness; then
    echo "Runtime credential refresh failed; previous credential restored." >&2
  else
    echo "Runtime credential refresh and rollback readiness both failed." >&2
  fi
}

case "$READINESS_RETRIES" in
  '' | *[!0-9]* | 0)
    echo "OKAWAK_BLOG_READINESS_RETRIES must be a positive integer." >&2
    exit 1
    ;;
esac

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

if [ -f "$RUNTIME_CREDENTIAL_FILE" ] && cmp -s "$tmp" "$RUNTIME_CREDENTIAL_FILE"; then
  chmod 600 "$RUNTIME_CREDENTIAL_FILE"
  rm -f "$tmp"
  trap - EXIT
  echo "$(date '+%F %T') runtime credentials unchanged"
  exit 0
fi

previous_file=""
if [ -f "$RUNTIME_CREDENTIAL_FILE" ]; then
  previous_file=$(mktemp "$RUNTIME_CREDENTIAL_DIR/.credentials.previous.XXXXXX")
  cp "$RUNTIME_CREDENTIAL_FILE" "$previous_file"
  chmod 600 "$previous_file"
fi

mv -f "$tmp" "$RUNTIME_CREDENTIAL_FILE"
trap - EXIT

# 4. credentialが変わった場合だけLeptos serviceを再起動し、S3 readinessを確認する。
# 初回移行ではdeploy前に省略できる。
if [ "${OKAWAK_BLOG_SKIP_RESTART:-0}" != "1" ]; then
  if ! sudo -n systemctl restart "$SERVICE_NAME" || ! check_readiness; then
    rollback_runtime_credential "$previous_file"
    exit 1
  fi
fi

if [ -n "$previous_file" ]; then
  rm -f "$previous_file"
fi

echo "$(date '+%F %T') runtime credentials updated"
