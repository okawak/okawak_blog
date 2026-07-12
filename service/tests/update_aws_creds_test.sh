#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
script="$repo_root/service/update_aws_creds.sh"
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

mode_of() {
  stat -c '%a' "$1" 2>/dev/null || stat -f '%Lp' "$1"
}

mock_bin="$work_dir/bin"
mkdir -p "$mock_bin"

cat >"$mock_bin/aws" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' '{"aws_access_key_id":"AKIATEST","aws_secret_access_key":"test-secret"}'
EOF

cat >"$mock_bin/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "$1" = "systemctl" ] && [ "$2" = "restart" ] && [ "$3" = "okawak_blog" ]; then
  exit 0
fi
echo "unexpected sudo command: $*" >&2
exit 1
EOF

chmod +x "$mock_bin/aws" "$mock_bin/sudo"

runtime_dir="$work_dir/existing-runtime"
mkdir -m 755 "$runtime_dir"
mode_before=$(mode_of "$runtime_dir")

PATH="$mock_bin:$PATH" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  OKAWAK_BLOG_SKIP_RESTART=1 \
  "$script"

test "$(mode_of "$runtime_dir")" = "$mode_before"
test "$(mode_of "$runtime_dir/credentials")" = "600"
grep -qx '\[blog-s3\]' "$runtime_dir/credentials"
grep -qx 'aws_access_key_id     = AKIATEST' "$runtime_dir/credentials"
grep -qx 'aws_secret_access_key = test-secret' "$runtime_dir/credentials"

shared_dir_mode=$(mode_of /tmp)
if PATH="$mock_bin:$PATH" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="/tmp/okawak-blog-unsafe-credentials" \
  OKAWAK_BLOG_SKIP_RESTART=1 \
  "$script"; then
  echo "shared parent directory should have been rejected" >&2
  exit 1
fi
test "$(mode_of /tmp)" = "$shared_dir_mode"

new_runtime_file="$work_dir/new-runtime/aws/credentials"
PATH="$mock_bin:$PATH" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$new_runtime_file" \
  OKAWAK_BLOG_SKIP_RESTART=1 \
  "$script"

test "$(mode_of "$work_dir/new-runtime/aws")" = "700"
test "$(mode_of "$new_runtime_file")" = "600"
