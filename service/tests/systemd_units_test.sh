#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
service_unit="$repo_root/service/okawak_blog.service"

grep -qx 'User=okawak' "$service_unit"
grep -qx 'Environment=AWS_PROFILE=blog-s3' "$service_unit"
grep -qx 'Environment=AWS_SHARED_CREDENTIALS_FILE=/var/lib/okawak_blog/aws/credentials' "$service_unit"
grep -qx 'StateDirectory=okawak_blog' "$service_unit"
grep -qx 'ProtectHome=true' "$service_unit"
