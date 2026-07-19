#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
service_unit="$repo_root/service/okawak_blog.service"
cloudflared_unit="$repo_root/service/cloudflared.service"

grep -qx 'User=okawak' "$service_unit"
grep -qx 'Environment=AWS_PROFILE=blog-s3' "$service_unit"
grep -qx 'Environment=AWS_CONFIG_FILE=/etc/okawak_blog/aws/config' "$service_unit"
grep -qx 'Environment=AWS_EC2_METADATA_DISABLED=true' "$service_unit"
if grep -q '^Environment=AWS_SHARED_CREDENTIALS_FILE=' "$service_unit"; then
  echo "production service must not use static AWS credentials" >&2
  exit 1
fi
grep -qx 'StateDirectory=okawak_blog' "$service_unit"
grep -qx 'ProtectHome=true' "$service_unit"

grep -qx 'Wants=network-online.target okawak_blog.service' "$cloudflared_unit"
grep -qx 'After=network-online.target okawak_blog.service' "$cloudflared_unit"
grep -qx 'User=cloudflared' "$cloudflared_unit"
grep -qx 'Group=cloudflared' "$cloudflared_unit"
grep -qx 'ExecStartPre=/usr/bin/test -r /etc/cloudflared/token' "$cloudflared_unit"
grep -qx 'ExecStart=/usr/local/bin/cloudflared tunnel --no-autoupdate run --token-file /etc/cloudflared/token' "$cloudflared_unit"
grep -qx 'Restart=on-failure' "$cloudflared_unit"
grep -qx 'NoNewPrivileges=true' "$cloudflared_unit"
grep -qx 'ProtectSystem=strict' "$cloudflared_unit"
grep -qx 'ProtectHome=true' "$cloudflared_unit"

if grep -Eq -- '--token([=[:space:]])' "$cloudflared_unit"; then
  echo "cloudflared service must read its token from a protected file" >&2
  exit 1
fi

if grep -q '^Environment=.*TOKEN' "$cloudflared_unit"; then
  echo "cloudflared service must not embed its token in an environment variable" >&2
  exit 1
fi
