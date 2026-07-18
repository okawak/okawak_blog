#!/usr/bin/env bash
set -euo pipefail

SOURCE_PROFILE="${OKAWAK_BLOG_BOOTSTRAP_SOURCE_PROFILE:-blog-s3}"
DESTINATION_PROFILE="${OKAWAK_BLOG_RUNTIME_PROFILE:-blog-s3}"
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

prepare_runtime_credential_dir

# Read the already-working static profile without printing its credential.
access_key_id=$(aws configure get aws_access_key_id --profile "$SOURCE_PROFILE")
secret_access_key=$(aws configure get aws_secret_access_key --profile "$SOURCE_PROFILE")
session_token=$(aws configure get aws_session_token --profile "$SOURCE_PROFILE" || true)

if [ -z "$access_key_id" ] || [ -z "$secret_access_key" ]; then
  echo "The bootstrap source profile must contain a static access key: $SOURCE_PROFILE" >&2
  exit 1
fi

if [ -n "$session_token" ]; then
  echo "The bootstrap source must be a long-lived fallback profile, not temporary credentials." >&2
  exit 1
fi

tmp=$(mktemp "$RUNTIME_CREDENTIAL_DIR/.credentials.XXXXXX")
trap 'rm -f "$tmp"' EXIT

cat >"$tmp" <<EOF
[$DESTINATION_PROFILE]
aws_access_key_id     = $access_key_id
aws_secret_access_key = $secret_access_key
EOF

chmod 600 "$tmp"

if [ -f "$RUNTIME_CREDENTIAL_FILE" ]; then
  if cmp -s "$tmp" "$RUNTIME_CREDENTIAL_FILE"; then
    rm -f "$tmp"
    trap - EXIT
    echo "Runtime credentials already match the source profile."
    exit 0
  fi

  echo "Refusing to replace an existing runtime credential file: $RUNTIME_CREDENTIAL_FILE" >&2
  echo "Resolve the existing file explicitly before running bootstrap again." >&2
  exit 1
fi

mv "$tmp" "$RUNTIME_CREDENTIAL_FILE"
trap - EXIT

echo "Runtime credentials bootstrapped from profile '$SOURCE_PROFILE'."
