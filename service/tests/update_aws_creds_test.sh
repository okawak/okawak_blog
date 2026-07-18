#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
script="$repo_root/service/update_aws_creds.sh"
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

mode_of() {
  stat -c '%a' "$1" 2>/dev/null || stat -f '%Lp' "$1"
}

mtime_of() {
  stat -c '%Y' "$1" 2>/dev/null || stat -f '%m' "$1"
}

mock_bin="$work_dir/bin"
mkdir -p "$mock_bin"

cat >"$mock_bin/aws" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "${MOCK_AWS_SECRET:?}"
EOF

cat >"$mock_bin/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${MOCK_SUDO_LOG:?}"
if [ "$1" = "-n" ] && [ "$2" = "systemctl" ] && [ "$3" = "restart" ] && [ "$4" = "okawak_blog" ]; then
  exit 0
fi
echo "unexpected sudo command: $*" >&2
exit 1
EOF

cat >"$mock_bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${MOCK_CURL_LOG:?}"
call_count=$(wc -l <"$MOCK_CURL_LOG" | tr -d ' ')
if [ "$call_count" -le "${MOCK_CURL_FAIL_COUNT:-0}" ]; then
  exit 1
fi
exit 0
EOF

cat >"$mock_bin/sleep" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF

chmod +x "$mock_bin/aws" "$mock_bin/sudo" "$mock_bin/curl" "$mock_bin/sleep"

secret_a='{"aws_access_key_id":"AKIATESTA","aws_secret_access_key":"test-secret-a"}'
secret_b='{"aws_access_key_id":"AKIATESTB","aws_secret_access_key":"test-secret-b"}'
secret_c='{"aws_access_key_id":"AKIATESTC","aws_secret_access_key":"test-secret-c"}'
sudo_log="$work_dir/sudo.log"
curl_log="$work_dir/curl.log"
touch "$sudo_log" "$curl_log"

runtime_dir="$work_dir/existing-runtime"
mkdir -m 755 "$runtime_dir"
mode_before=$(mode_of "$runtime_dir")

PATH="$mock_bin:$PATH" \
  MOCK_AWS_SECRET="$secret_a" \
  MOCK_SUDO_LOG="$sudo_log" \
  MOCK_CURL_LOG="$curl_log" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  OKAWAK_BLOG_SKIP_RESTART=1 \
  "$script"

test "$(mode_of "$runtime_dir")" = "$mode_before"
test "$(mode_of "$runtime_dir/credentials")" = "600"
grep -qx '\[blog-s3\]' "$runtime_dir/credentials"
grep -qx 'aws_access_key_id     = AKIATESTA' "$runtime_dir/credentials"
grep -qx 'aws_secret_access_key = test-secret-a' "$runtime_dir/credentials"

# An unchanged secret must not rewrite the file or restart the service.
touch -t 202001010000 "$runtime_dir/credentials"
mtime_before=$(mtime_of "$runtime_dir/credentials")
PATH="$mock_bin:$PATH" \
  MOCK_AWS_SECRET="$secret_a" \
  MOCK_SUDO_LOG="$sudo_log" \
  MOCK_CURL_LOG="$curl_log" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  "$script"
test "$(mtime_of "$runtime_dir/credentials")" = "$mtime_before"
test ! -s "$sudo_log"
test ! -s "$curl_log"

# A changed secret must atomically replace the file, restart, and pass readiness.
PATH="$mock_bin:$PATH" \
  MOCK_AWS_SECRET="$secret_b" \
  MOCK_SUDO_LOG="$sudo_log" \
  MOCK_CURL_LOG="$curl_log" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  OKAWAK_BLOG_READINESS_RETRIES=2 \
  OKAWAK_BLOG_READINESS_RETRY_INTERVAL_SECONDS=0 \
  "$script"
grep -qx -- '-n systemctl restart okawak_blog' "$sudo_log"
grep -q '/api/ready' "$curl_log"
grep -qx 'aws_access_key_id     = AKIATESTB' "$runtime_dir/credentials"
test "$(mode_of "$runtime_dir/credentials")" = "600"

# Readiness failure after a changed secret must make the refresh fail visibly.
: >"$sudo_log"
: >"$curl_log"
if PATH="$mock_bin:$PATH" \
  MOCK_AWS_SECRET="$secret_c" \
  MOCK_SUDO_LOG="$sudo_log" \
  MOCK_CURL_LOG="$curl_log" \
  MOCK_CURL_FAIL_COUNT=2 \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  OKAWAK_BLOG_READINESS_RETRIES=2 \
  OKAWAK_BLOG_READINESS_RETRY_INTERVAL_SECONDS=0 \
  "$script"; then
  echo "readiness failure should fail the credential refresh" >&2
  exit 1
fi
test "$(wc -l <"$curl_log" | tr -d ' ')" = "3"
test "$(wc -l <"$sudo_log" | tr -d ' ')" = "2"
grep -qx 'aws_access_key_id     = AKIATESTB' "$runtime_dir/credentials"
grep -qx 'aws_secret_access_key = test-secret-b' "$runtime_dir/credentials"

shared_dir_mode=$(mode_of /tmp)
if PATH="$mock_bin:$PATH" \
  MOCK_AWS_SECRET="$secret_a" \
  MOCK_SUDO_LOG="$sudo_log" \
  MOCK_CURL_LOG="$curl_log" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="/tmp/okawak-blog-unsafe-credentials" \
  OKAWAK_BLOG_SKIP_RESTART=1 \
  "$script"; then
  echo "shared parent directory should have been rejected" >&2
  exit 1
fi
test "$(mode_of /tmp)" = "$shared_dir_mode"

new_runtime_file="$work_dir/new-runtime/aws/credentials"
PATH="$mock_bin:$PATH" \
  MOCK_AWS_SECRET="$secret_a" \
  MOCK_SUDO_LOG="$sudo_log" \
  MOCK_CURL_LOG="$curl_log" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$new_runtime_file" \
  OKAWAK_BLOG_SKIP_RESTART=1 \
  "$script"

test "$(mode_of "$work_dir/new-runtime/aws")" = "700"
test "$(mode_of "$new_runtime_file")" = "600"
