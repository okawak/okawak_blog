#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
script="$repo_root/service/bootstrap_aws_credentials.sh"
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
printf '%s\n' "$*" >>"${MOCK_AWS_ARGS:?}"
case "$3" in
  aws_access_key_id) printf '%s\n' "${MOCK_AWS_ACCESS_KEY_ID:?}" ;;
  aws_secret_access_key) printf '%s\n' "${MOCK_AWS_SECRET_ACCESS_KEY:?}" ;;
  aws_session_token)
    if [ -n "${MOCK_AWS_SESSION_TOKEN:-}" ]; then
      printf '%s\n' "$MOCK_AWS_SESSION_TOKEN"
    else
      exit 1
    fi
    ;;
  *) echo "unexpected aws command: $*" >&2; exit 1 ;;
esac
EOF

chmod +x "$mock_bin/aws"

aws_args="$work_dir/aws-args"
runtime_dir="$work_dir/existing-runtime"
mkdir -m 755 "$runtime_dir"
mode_before=$(mode_of "$runtime_dir")

PATH="$mock_bin:$PATH" \
  MOCK_AWS_ARGS="$aws_args" \
  MOCK_AWS_ACCESS_KEY_ID="AKIATESTA" \
  MOCK_AWS_SECRET_ACCESS_KEY="test-secret-a" \
  OKAWAK_BLOG_BOOTSTRAP_SOURCE_PROFILE="existing-reader" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  "$script"

grep -qx 'configure get aws_access_key_id --profile existing-reader' "$aws_args"
grep -qx 'configure get aws_secret_access_key --profile existing-reader' "$aws_args"
grep -qx 'configure get aws_session_token --profile existing-reader' "$aws_args"
test "$(mode_of "$runtime_dir")" = "$mode_before"
test "$(mode_of "$runtime_dir/credentials")" = "600"
grep -qx '\[blog-s3\]' "$runtime_dir/credentials"
grep -qx 'aws_access_key_id     = AKIATESTA' "$runtime_dir/credentials"
grep -qx 'aws_secret_access_key = test-secret-a' "$runtime_dir/credentials"

# Re-running with the same profile is idempotent.
PATH="$mock_bin:$PATH" \
  MOCK_AWS_ARGS="$aws_args" \
  MOCK_AWS_ACCESS_KEY_ID="AKIATESTA" \
  MOCK_AWS_SECRET_ACCESS_KEY="test-secret-a" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  "$script"

# A different credential must not overwrite the fallback silently.
if PATH="$mock_bin:$PATH" \
  MOCK_AWS_ARGS="$aws_args" \
  MOCK_AWS_ACCESS_KEY_ID="AKIATESTB" \
  MOCK_AWS_SECRET_ACCESS_KEY="test-secret-b" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$runtime_dir/credentials" \
  "$script"; then
  echo "bootstrap should refuse to overwrite existing runtime credentials" >&2
  exit 1
fi
grep -qx 'aws_access_key_id     = AKIATESTA' "$runtime_dir/credentials"

# Temporary credentials would expire because this fallback has no refresh loop.
temporary_runtime_file="$work_dir/temporary-runtime/aws/credentials"
if PATH="$mock_bin:$PATH" \
  MOCK_AWS_ARGS="$aws_args" \
  MOCK_AWS_ACCESS_KEY_ID="ASIAtest" \
  MOCK_AWS_SECRET_ACCESS_KEY="test-secret-b" \
  MOCK_AWS_SESSION_TOKEN="test-token" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$temporary_runtime_file" \
  "$script"; then
  echo "bootstrap should reject temporary credentials" >&2
  exit 1
fi
test ! -f "$temporary_runtime_file"

# A new runtime directory is private.
new_runtime_file="$work_dir/new-runtime/aws/credentials"
PATH="$mock_bin:$PATH" \
  MOCK_AWS_ARGS="$aws_args" \
  MOCK_AWS_ACCESS_KEY_ID="AKIATESTA" \
  MOCK_AWS_SECRET_ACCESS_KEY="test-secret-a" \
  OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE="$new_runtime_file" \
  "$script"

test "$(mode_of "$work_dir/new-runtime/aws")" = "700"
test "$(mode_of "$new_runtime_file")" = "600"
