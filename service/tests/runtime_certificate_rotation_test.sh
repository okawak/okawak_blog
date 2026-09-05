#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
activation_script="$repo_root/scripts/activate_runtime_certificate.sh"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT

fail() {
  echo "runtime-certificate-rotation-test: $*" >&2
  exit 1
}

write_command_stubs() {
  local stub_dir="$1"

  mkdir -p "$stub_dir"
  cat >"$stub_dir/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-v" ]]; then
  exit 0
fi
if [[ "${1:-}" == "-u" ]]; then
  shift 2
fi
exec "$@"
EOF
  cat >"$stub_dir/install" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
args=()
while (($# > 0)); do
  case "$1" in
    -o|-g)
      shift 2
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done
/usr/bin/install "${args[@]}"
EOF
  cat >"$stub_dir/chown" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat >"$stub_dir/sed" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == "-E" && "$2" == "-i" ]]
shift 2
expressions=()
while [[ "${1:-}" == "-e" ]]; do
  expressions+=("-e" "$2")
  shift 2
done
input_file="$1"
temporary_file="$input_file.tmp"
/usr/bin/sed -E "${expressions[@]}" "$input_file" >"$temporary_file"
mv "$temporary_file" "$input_file"
EOF
  cat >"$stub_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "is-active" ]]; then
  [[ "${STUB_SERVICE_ACTIVE:-false}" == "true" ]]
  exit
fi
printf '%s\n' "$*" >>"$STUB_SYSTEMCTL_LOG"
EOF
  cat >"$stub_dir/aws" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ -r "$AWS_CONFIG_FILE" ]]
grep -F -- '--certificate ' "$AWS_CONFIG_FILE" >/dev/null
grep -F -- '--private-key ' "$AWS_CONFIG_FILE" >/dev/null
if [[ "$1" == "sts" ]]; then
  echo 'arn:aws:sts::123456789012:assumed-role/okawak-blog-runtime-role/test'
else
  echo '123'
fi
EOF
  cat >"$stub_dir/curl" <<'EOF'
#!/usr/bin/env bash
if [[ "${STUB_CURL_FAIL:-false}" == "true" ]]; then
  exit 22
fi
exit 0
EOF
  chmod +x "$stub_dir"/*
}

create_ca() {
  local case_dir="$1"

  openssl genpkey \
    -algorithm EC \
    -pkeyopt ec_paramgen_curve:P-256 \
    -out "$case_dir/ca-key.pem" >/dev/null 2>&1
  openssl req \
    -x509 \
    -new \
    -sha256 \
    -key "$case_dir/ca-key.pem" \
    -out "$case_dir/ca-cert.pem" \
    -days 30 \
    -subj '/O=test/CN=test-ca' >/dev/null 2>&1
}

create_client_certificate() {
  local case_dir="$1"
  local name="$2"

  openssl genpkey \
    -algorithm EC \
    -pkeyopt ec_paramgen_curve:P-256 \
    -out "$case_dir/$name-key.pem" >/dev/null 2>&1
  openssl req \
    -new \
    -key "$case_dir/$name-key.pem" \
    -out "$case_dir/$name.csr" \
    -subj '/O=okawak/CN=okawak-blog-vps' >/dev/null 2>&1
  openssl x509 \
    -req \
    -in "$case_dir/$name.csr" \
    -CA "$case_dir/ca-cert.pem" \
    -CAkey "$case_dir/ca-key.pem" \
    -CAcreateserial \
    -out "$case_dir/$name-cert.pem" \
    -days 14 \
    -sha256 >/dev/null 2>&1
}

prepare_case() {
  local case_dir="$1"
  local stamp="$2"

  mkdir -p "$case_dir/certificates" "$case_dir/upload" "$case_dir/stubs"
  write_command_stubs "$case_dir/stubs"
  create_ca "$case_dir"
  create_client_certificate "$case_dir" old
  create_client_certificate "$case_dir" new

  cp "$case_dir/old-cert.pem" "$case_dir/certificates/client-cert.pem"
  cp "$case_dir/old-key.pem" "$case_dir/certificates/client-key.pem"
  cp "$case_dir/new-cert.pem" "$case_dir/upload/vps-client-cert-$stamp.pem"
  cp "$case_dir/new-key.pem" "$case_dir/upload/vps-client-key-$stamp.pem"
  cp "$activation_script" "$case_dir/upload/activate_runtime_certificate.sh"

  cat >"$case_dir/certificates/config" <<EOF
[profile blog-s3]
region = ap-northeast-1
credential_process = /usr/local/bin/aws_signing_helper credential-process --certificate $case_dir/certificates/client-cert.pem --private-key $case_dir/certificates/client-key.pem --trust-anchor-arn test --profile-arn test --role-arn test
EOF
  : >"$case_dir/systemctl.log"
}

run_activation() {
  local case_dir="$1"
  local stamp="$2"
  local curl_fail="$3"

  PATH="$case_dir/stubs:$PATH" \
    ROTATION_STAMP="$stamp" \
    UPLOAD_DIR="$case_dir/upload" \
    ALLOW_CUSTOM_UPLOAD_DIR=1 \
    CERTIFICATE_DIR="$case_dir/certificates" \
    AWS_CONFIG_PATH="$case_dir/certificates/config" \
    SERVICE_USER="$(id -un)" \
    SERVICE_GROUP="$(id -gn)" \
    STUB_SERVICE_ACTIVE=true \
    STUB_CURL_FAIL="$curl_fail" \
    STUB_SYSTEMCTL_LOG="$case_dir/systemctl.log" \
    bash "$activation_script"
}

success_stamp='20260905T010101Z'
success_case="$test_root/success"
prepare_case "$success_case" "$success_stamp"
run_activation "$success_case" "$success_stamp" false
cmp -s "$success_case/new-cert.pem" "$success_case/certificates/client-cert.pem" \
  || fail "successful activation did not install the new certificate"
cmp -s "$success_case/new-key.pem" "$success_case/certificates/client-key.pem" \
  || fail "successful activation did not install the new private key"
[[ ! -e "$success_case/certificates/client-cert.pem.rollback-$success_stamp" ]] \
  || fail "successful activation retained the rollback certificate"
[[ ! -e "$success_case/certificates/config-$success_stamp" ]] \
  || fail "successful activation retained the candidate AWS config"

rollback_stamp='20260905T020202Z'
rollback_case="$test_root/rollback"
prepare_case "$rollback_case" "$rollback_stamp"
if run_activation "$rollback_case" "$rollback_stamp" true; then
  fail "failed readiness probes unexpectedly completed certificate activation"
fi
cmp -s "$rollback_case/old-cert.pem" "$rollback_case/certificates/client-cert.pem" \
  || fail "probe failure did not restore the old certificate"
cmp -s "$rollback_case/old-key.pem" "$rollback_case/certificates/client-key.pem" \
  || fail "probe failure did not restore the old private key"
grep -qx 'start okawak_blog.service' "$rollback_case/systemctl.log" \
  || fail "probe failure did not restart the previously active service"

echo "runtime-certificate-rotation-test: all cases passed"
