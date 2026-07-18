#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
service_unit="$repo_root/service/okawak_blog-aws-credentials.service"
timer_unit="$repo_root/service/okawak_blog-aws-credentials.timer"

grep -qx 'User=okawak' "$service_unit"
grep -qx 'TimeoutStartSec=2min' "$service_unit"
grep -qx 'ExecStart=/usr/local/bin/update_aws_creds.sh' "$service_unit"
grep -qx 'Environment=OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE=/var/lib/okawak_blog/aws/credentials' "$service_unit"
grep -qx 'ProtectHome=read-only' "$service_unit"

grep -qx 'OnBootSec=10min' "$timer_unit"
grep -qx 'OnCalendar=\*-\*-\* 04:05:00' "$timer_unit"
grep -qx 'Persistent=true' "$timer_unit"
grep -qx 'Unit=okawak_blog-aws-credentials.service' "$timer_unit"
