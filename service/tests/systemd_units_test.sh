#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
service_unit="$repo_root/service/okawak_blog.service"

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
