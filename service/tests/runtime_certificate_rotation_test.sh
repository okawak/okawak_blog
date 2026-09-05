#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
activation_script="$repo_root/scripts/activate_runtime_certificate.sh"
rotation_script="$repo_root/scripts/rotate_runtime_certificate.sh"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT

fail() {
  echo "runtime-certificate-rotation-test: $*" >&2
  exit 1
}

write_command_stubs() {
  local stub_dir="$1"

  mkdir -p "$stub_dir"
  cat >"$stub_dir/signal-helper.sh" <<'EOF'
signal_activation() {
  if [[ "$1" == service-stopped && -e "${STUB_SIGNAL_SENT:-}" ]]; then
    # Exercise repeated signals while the rollback is already running.
    : >"$STUB_SIGNAL_SENT.repeated"
    kill -s HUP "$PPID"
    kill -s INT "$PPID"
    kill -s TERM "$PPID"
    return 0
  fi
  [[ "${STUB_SIGNAL_POINT:-}" == "$1" ]] || return 0
  [[ ! -e "$STUB_SIGNAL_SENT" ]] || return 0
  : >"$STUB_SIGNAL_SENT"
  kill -s "$STUB_SIGNAL" "$PPID"
}
EOF
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
source_file="${args[${#args[@]}-2]}"
destination_file="${args[${#args[@]}-1]}"
if [[ "$source_file" == "$CERTIFICATE_DIR/${STUB_RESTORE_FAILURE_FILE:-}.pem.rollback-$ROTATION_STAMP" ]]; then
  : >"$STUB_RESTORE_FAILURE_MARKER"
  printf 'incomplete restore\n' >"$destination_file"
  exit "${STUB_RESTORE_FAILURE_STATUS:-1}"
fi
/usr/bin/install "${args[@]}"
# shellcheck source=/dev/null
source "$(dirname "$0")/signal-helper.sh"
if [[ "${args[${#args[@]}-1]}" == "$CERTIFICATE_DIR/client-cert.pem" ]]; then
  signal_activation certificate-installed
fi
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
if [[ "${STUB_RECOVERY_SYSTEMCTL_FAILURE:-}" == "$1" ]] \
  && (( $(grep -c '^stop ' "$STUB_SYSTEMCTL_LOG") >= 2 )); then
  : >"$STUB_RESTORE_FAILURE_MARKER"
  exit 1
fi
# shellcheck source=/dev/null
source "$(dirname "$0")/signal-helper.sh"
if [[ "$1" == "stop" ]]; then
  signal_activation service-stopped
fi
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
# shellcheck source=/dev/null
source "$(dirname "$0")/signal-helper.sh"
signal_activation probes
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
    -subj '/O=test/CN=test-ca' \
    -addext 'basicConstraints=critical,CA:TRUE' \
    -addext 'keyUsage=critical,keyCertSign,cRLSign' >/dev/null 2>&1
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
    STUB_SIGNAL_SENT="$case_dir/signal-sent" \
    STUB_RESTORE_FAILURE_MARKER="$case_dir/restore-failed" \
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
[[ ! -e "$rollback_case/certificates/client-cert.pem.rollback-$rollback_stamp" \
  && ! -e "$rollback_case/certificates/client-key.pem.rollback-$rollback_stamp" ]] \
  || fail "successful rollback retained its backups"

for restore_status in 1 0; do
  for restore_file in client-cert client-key; do
    restore_case="$test_root/restore-$restore_file-status-$restore_status"
    restore_stamp='20260905T040404Z'
    prepare_case "$restore_case" "$restore_stamp"
    actual_status=0
    STUB_RESTORE_FAILURE_FILE="$restore_file" STUB_RESTORE_FAILURE_STATUS="$restore_status" \
      run_activation "$restore_case" "$restore_stamp" true \
      >"$restore_case/output.log" 2>&1 || actual_status=$?
    [[ -f "$restore_case/restore-failed" ]] || fail "restore failure was not injected"
    [[ "$actual_status" == 22 ]] || fail "restore failure lost the original probe exit status"
    cmp -s "$restore_case/old-cert.pem" \
      "$restore_case/certificates/client-cert.pem.rollback-$restore_stamp" \
      || fail "failed certificate restore deleted or damaged the rollback certificate"
    cmp -s "$restore_case/old-key.pem" \
      "$restore_case/certificates/client-key.pem.rollback-$restore_stamp" \
      || fail "failed private key restore deleted or damaged the rollback private key"
    [[ "$(tail -n 1 "$restore_case/systemctl.log")" == 'stop okawak_blog.service' ]] \
      || fail "failed restore restarted the service with an inconsistent pair"
    grep -Fq "client-cert.pem.rollback-$restore_stamp" "$restore_case/output.log" \
      || fail "failed restore did not report the preserved certificate path"
    grep -Fq "client-key.pem.rollback-$restore_stamp" "$restore_case/output.log" \
      || fail "failed restore did not report the preserved private key path"
    [[ ! -e "$restore_case/certificates/config-$restore_stamp" && ! -d "$restore_case/upload" ]] \
      || fail "failed restore left temporary rotation files"
    echo "runtime-certificate-rotation-test: $restore_file restore status $restore_status preserved the old pair"
  done
done

for recovery_command in stop start; do
  recovery_case="$test_root/recovery-$recovery_command"
  recovery_stamp='20260905T050505Z'
  prepare_case "$recovery_case" "$recovery_stamp"
  actual_status=0
  STUB_RECOVERY_SYSTEMCTL_FAILURE="$recovery_command" \
    run_activation "$recovery_case" "$recovery_stamp" true \
    >"$recovery_case/output.log" 2>&1 || actual_status=$?
  [[ -f "$recovery_case/restore-failed" && "$actual_status" == 22 ]] \
    || fail "recovery $recovery_command failure was not exercised"
  cmp -s "$recovery_case/old-cert.pem" \
    "$recovery_case/certificates/client-cert.pem.rollback-$recovery_stamp" \
    || fail "recovery $recovery_command failure lost the rollback certificate"
  cmp -s "$recovery_case/old-key.pem" \
    "$recovery_case/certificates/client-key.pem.rollback-$recovery_stamp" \
    || fail "recovery $recovery_command failure lost the rollback private key"
  if [[ "$recovery_command" == stop ]]; then
    # Do not overwrite a pair that the running service could still be using.
    expected_pair=new
  else
    expected_pair=old
  fi
  cmp -s "$recovery_case/$expected_pair-cert.pem" "$recovery_case/certificates/client-cert.pem" \
    || fail "recovery $recovery_command failure left the wrong active certificate"
  cmp -s "$recovery_case/$expected_pair-key.pem" "$recovery_case/certificates/client-key.pem" \
    || fail "recovery $recovery_command failure left the wrong active private key"
  echo "runtime-certificate-rotation-test: recovery $recovery_command failure preserved the old pair"
done

for signal_name in HUP INT TERM; do
  case "$signal_name" in
    HUP) expected_status=129 ;;
    INT) expected_status=130 ;;
    TERM) expected_status=143 ;;
  esac
  for signal_point in service-stopped certificate-installed probes; do
    signal_case="$test_root/signal-$signal_name-$signal_point"
    signal_stamp='20260905T030303Z'
    prepare_case "$signal_case" "$signal_stamp"
    actual_status=0
    STUB_SIGNAL="$signal_name" STUB_SIGNAL_POINT="$signal_point" \
      run_activation "$signal_case" "$signal_stamp" false \
      >"$signal_case/output.log" 2>&1 || actual_status=$?
    [[ -f "$signal_case/signal-sent" ]] || fail "$signal_name was not sent at $signal_point"
    [[ -f "$signal_case/signal-sent.repeated" ]] || fail "rollback did not receive repeated signals"
    [[ "$actual_status" == "$expected_status" ]] \
      || fail "$signal_name at $signal_point: expected status $expected_status, got $actual_status"
    cmp -s "$signal_case/old-cert.pem" "$signal_case/certificates/client-cert.pem" \
      || fail "$signal_name at $signal_point did not restore the old certificate"
    cmp -s "$signal_case/old-key.pem" "$signal_case/certificates/client-key.pem" \
      || fail "$signal_name at $signal_point did not restore the old private key"
    [[ "$(tail -n 1 "$signal_case/systemctl.log")" == 'start okawak_blog.service' ]] \
      || fail "$signal_name at $signal_point left the service stopped"
    [[ ! -e "$signal_case/certificates/config-$signal_stamp" && ! -d "$signal_case/upload" ]] \
      || fail "$signal_name at $signal_point left temporary rotation files"
    echo "runtime-certificate-rotation-test: $signal_name at $signal_point restored the old pair and service"
  done
done

test_subject_cn() {
  local case_name="$1"
  local expected_cn="$2"
  local case_dir="$test_root/cn-$case_name"

  mkdir -p "$case_dir/stubs" "$case_dir/uploaded"
  create_ca "$case_dir"
  create_client_certificate "$case_dir" seed
  cat >"$case_dir/stubs/ssh" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == '-tt' && -n "${STUB_LOCAL_SIGNAL:-}" ]]; then
  kill -s "$STUB_LOCAL_SIGNAL" "$PPID"
fi
exit 0
EOF
  cat >"$case_dir/stubs/scp" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cp "$1" "$STUB_UPLOADED_DIR/cert.pem"
cp "$2" "$STUB_UPLOADED_DIR/key.pem"
EOF
  chmod +x "$case_dir/stubs/ssh" "$case_dir/stubs/scp"
  local actual_status=0
  PATH="$case_dir/stubs:$PATH" \
    OKAWAK_BLOG_PKI_DIR="$case_dir" \
    STUB_UPLOADED_DIR="$case_dir/uploaded" \
    bash "$rotation_script" test-vps >"$case_dir/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" == "${STUB_ROTATION_STATUS:-0}" ]] \
    || fail "$case_name rotation: unexpected exit status $actual_status"

  openssl verify -purpose sslclient -CAfile "$case_dir/ca-cert.pem" \
    "$case_dir/uploaded/cert.pem" >/dev/null
  local subject
  subject="$(openssl x509 -in "$case_dir/uploaded/cert.pem" \
    -noout -subject -nameopt sep_multiline,sname,utf8)"
  [[ "$(printf '%s\n' "$subject" | sed -n 's/^ *CN=//p')" == "$expected_cn" ]] \
    || fail "issued certificate CN does not match $expected_cn: $subject"
  [[ ! -d "$case_dir/.certificate-rotation.lock" ]] || fail "rotation lock was retained"
  echo "runtime-certificate-rotation-test: $case_name CN matches the issued certificate"
}

unset OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN
test_subject_cn default okawak-blog-vps
OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN=custom-blog-vps \
  test_subject_cn custom custom-blog-vps
OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN='blue/team+CN=unexpected\node' \
  test_subject_cn escaped 'blue/team+CN=unexpected\node'

for signal_name in HUP INT TERM; do
  case "$signal_name" in
    HUP) expected_status=129 ;;
    INT) expected_status=130 ;;
    TERM) expected_status=143 ;;
  esac
  STUB_LOCAL_SIGNAL="$signal_name" STUB_ROTATION_STATUS="$expected_status" \
    test_subject_cn "local-signal-$signal_name" okawak-blog-vps
done

for invalid_cn in '' $'blog\nCN=injected'; do
  if OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN="$invalid_cn" \
    OKAWAK_BLOG_PKI_DIR="$test_root/not-created" \
    bash "$rotation_script" test-vps >"$test_root/invalid-cn.log" 2>&1; then
    fail "empty or multiline CN was accepted"
  fi
  grep -q 'OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN must be' "$test_root/invalid-cn.log" \
    || fail "invalid CN was not rejected before accessing the CA"
done

echo "runtime-certificate-rotation-test: all cases passed"
