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
if [[ "$1" == "is-active" || "$1" == "show" ]]; then
  if [[ "${STUB_SERVICE_QUERY_STATUS:-0}" != 0 ]]; then
    echo 'simulated service manager query failure' >&2
    exit "$STUB_SERVICE_QUERY_STATUS"
  fi
  if [[ "$1" == "is-active" ]]; then
    [[ "${STUB_SERVICE_STATE:-active}" == active ]]
    exit
  fi
  if [[ "${STUB_SERVICE_PROPERTIES+set}" == set ]]; then
    printf '%s\n' "$STUB_SERVICE_PROPERTIES"
  else
    printf 'LoadState=%s\nActiveState=%s\n' "${STUB_SERVICE_LOAD_STATE:-loaded}" "${STUB_SERVICE_STATE:-active}"
  fi
  exit 0
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
if [[ "${AWS_PAGER-unset}" != "" ]]; then
  echo 'AWS pager must be explicitly disabled for unattended validation' >&2
  exit 1
fi
[[ -r "$AWS_CONFIG_FILE" ]]
grep -F -- '--certificate ' "$AWS_CONFIG_FILE" >/dev/null
grep -F -- '--private-key ' "$AWS_CONFIG_FILE" >/dev/null
phase=candidate
if [[ "$AWS_CONFIG_FILE" == "$CERTIFICATE_DIR/config" ]]; then
  phase=active
fi
if [[ "${STUB_AWS_FAIL_AT:-}" == "$phase-$1" ]]; then
  echo "simulated AWS failure at $phase-$1" >&2
  exit 42
fi
if [[ "$1" == "sts" ]]; then
  echo 'arn:aws:sts::123456789012:assumed-role/okawak-blog-runtime-role/test'
else
  echo '123'
fi
EOF
  cat >"$stub_dir/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=/dev/null
source "$(dirname "$0")/signal-helper.sh"
signal_activation probes
probe_url="${!#}"
probe="${probe_url##*/}"
printf '%s\n' "$probe" >>"$STUB_CURL_LOG"
printf '%s\n' "$*" >>"$STUB_CURL_ARGUMENTS_LOG"
case "$probe" in
  health) failures="${STUB_HEALTH_FAILURES:-0}" ;;
  ready) failures="${STUB_READY_FAILURES:-0}" ;;
  *) exit 1 ;;
esac
if [[ "${STUB_CURL_FAIL:-false}" == "true" ]] \
  || (( $(grep -c "^$probe$" "$STUB_CURL_LOG") <= failures )); then
  exit "${STUB_PROBE_FAILURE_STATUS:-22}"
fi
exit 0
EOF
  cat >"$stub_dir/sleep" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$STUB_SLEEP_LOG"
# shellcheck source=/dev/null
source "$(dirname "$0")/signal-helper.sh"
signal_activation probe-wait
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
  : >"$case_dir/curl.log"
  : >"$case_dir/curl-arguments.log"
  : >"$case_dir/sleep.log"
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
    AWS_PAGER=must-not-run-certificate-pager \
    STUB_SERVICE_STATE="${STUB_SERVICE_STATE:-active}" \
    STUB_CURL_FAIL="$curl_fail" \
    STUB_CURL_LOG="$case_dir/curl.log" \
    STUB_CURL_ARGUMENTS_LOG="$case_dir/curl-arguments.log" \
    STUB_SLEEP_LOG="$case_dir/sleep.log" \
    STUB_SIGNAL_SENT="$case_dir/signal-sent" \
    STUB_RESTORE_FAILURE_MARKER="$case_dir/restore-failed" \
    STUB_SYSTEMCTL_LOG="$case_dir/systemctl.log" \
    bash "$activation_script"
}

assert_single_recovery() {
  local case_dir="$1"
  local stamp="$2"
  local phase="$3"

  [[ "$(grep -c 'restoring the previous certificate' "$case_dir/output.log")" == 1 ]] \
    || fail "$phase failure ran recovery more than once"
  cmp -s "$case_dir/old-cert.pem" "$case_dir/certificates/client-cert.pem" \
    || fail "$phase failure did not preserve the old certificate"
  cmp -s "$case_dir/old-key.pem" "$case_dir/certificates/client-key.pem" \
    || fail "$phase failure did not preserve the old private key"
  if [[ "$phase" == active ]]; then
    [[ "$(cat "$case_dir/systemctl.log")" == \
      $'stop okawak_blog.service\nstart okawak_blog.service\nstop okawak_blog.service\nstart okawak_blog.service' ]] \
      || fail "active failure did not restore the service exactly once"
  else
    [[ ! -s "$case_dir/systemctl.log" ]] || fail "preflight failure changed the running service"
  fi
  [[ ! -e "$case_dir/certificates/client-cert.pem.rollback-$stamp" \
    && ! -e "$case_dir/certificates/client-key.pem.rollback-$stamp" \
    && ! -e "$case_dir/certificates/config-$stamp" \
    && ! -e "$case_dir/certificates/client-cert-$stamp.pem" \
    && ! -e "$case_dir/certificates/client-key-$stamp.pem" \
    && ! -d "$case_dir/upload" ]] || fail "$phase failure left temporary rotation files"
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
[[ ! -s "$success_case/sleep.log" ]] || fail "healthy service was unnecessarily retried"

for query_status in 1 3; do
  query_case="$test_root/service-query-$query_status"
  query_stamp='20260905T101010Z'
  prepare_case "$query_case" "$query_stamp"
  actual_status=0
  STUB_SERVICE_QUERY_STATUS="$query_status" run_activation "$query_case" "$query_stamp" false \
    >"$query_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" == "$query_status" ]] || fail "service query failure did not abort with its exit status"
  assert_single_recovery "$query_case" "$query_stamp" preflight
done

for invalid_state in activating deactivating reloading failed unknown ''; do
  state_case="$test_root/service-state-$invalid_state"
  state_stamp='20260905T111111Z'
  prepare_case "$state_case" "$state_stamp"
  actual_status=0
  STUB_SERVICE_PROPERTIES="$(printf 'LoadState=loaded\nActiveState=%s' "$invalid_state")" \
    run_activation "$state_case" "$state_stamp" false \
      >"$state_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" != 0 ]] || fail "unsafe service state was accepted: $invalid_state"
  assert_single_recovery "$state_case" "$state_stamp" preflight
done

for load_state in not-found masked error ''; do
  load_case="$test_root/service-load-$load_state"
  load_stamp='20260905T121212Z'
  prepare_case "$load_case" "$load_stamp"
  actual_status=0
  STUB_SERVICE_PROPERTIES="$(printf 'ActiveState=inactive\nLoadState=%s' "$load_state")" \
    run_activation "$load_case" "$load_stamp" false \
      >"$load_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" != 0 ]] || fail "unloaded service was accepted: $load_state"
  assert_single_recovery "$load_case" "$load_stamp" preflight
done

inactive_case="$test_root/service-inactive"
inactive_stamp='20260905T131313Z'
prepare_case "$inactive_case" "$inactive_stamp"
STUB_SERVICE_STATE=inactive STUB_SERVICE_PROPERTIES=$'ActiveState=inactive\nLoadState=loaded' \
  run_activation "$inactive_case" "$inactive_stamp" false >"$inactive_case/output.log" 2>&1
cmp -s "$inactive_case/new-cert.pem" "$inactive_case/certificates/client-cert.pem" \
  || fail "inactive service did not receive the new certificate"
cmp -s "$inactive_case/new-key.pem" "$inactive_case/certificates/client-key.pem" \
  || fail "inactive service did not receive the new private key"
[[ ! -s "$inactive_case/systemctl.log" && ! -s "$inactive_case/curl.log" ]] \
  || fail "inactive service was started or probed"
echo 'runtime-certificate-rotation-test: service query errors and unsafe states abort before activation'

for delayed_probe in health ready both; do
  delayed_case="$test_root/delayed-$delayed_probe"
  delayed_stamp='20260905T080808Z'
  prepare_case "$delayed_case" "$delayed_stamp"
  case "$delayed_probe" in
    health) health_failures=2; ready_failures=0; failure_status=7 ;;
    ready) health_failures=0; ready_failures=2; failure_status=22 ;;
    # Recovery on the last allowed attempt must still succeed.
    both) health_failures=7; ready_failures=7; failure_status=28 ;;
  esac
  STUB_HEALTH_FAILURES="$health_failures" STUB_READY_FAILURES="$ready_failures" \
    STUB_PROBE_FAILURE_STATUS="$failure_status" \
    run_activation "$delayed_case" "$delayed_stamp" false \
      >"$delayed_case/output.log" 2>&1 || fail "transient $delayed_probe failure rolled back"
  cmp -s "$delayed_case/new-cert.pem" "$delayed_case/certificates/client-cert.pem" \
    || fail "$delayed_probe delay did not retain the new certificate"
  cmp -s "$delayed_case/new-key.pem" "$delayed_case/certificates/client-key.pem" \
    || fail "$delayed_probe delay did not retain the new private key"
  [[ "$(cat "$delayed_case/systemctl.log")" == $'stop okawak_blog.service\nstart okawak_blog.service' ]] \
    || fail "$delayed_probe delay restarted the service more than once"
  [[ "$(grep -c '^health$' "$delayed_case/curl.log")" == "$((health_failures + ready_failures + 1))" \
    && "$(grep -c '^ready$' "$delayed_case/curl.log")" == "$((ready_failures + 1))" \
    && "$(grep -c '^1$' "$delayed_case/sleep.log")" == "$((health_failures + ready_failures))" ]] \
    || fail "$delayed_probe delay did not retry both probes at one-second intervals"
  [[ ! -e "$delayed_case/certificates/client-cert.pem.rollback-$delayed_stamp" \
    && ! -e "$delayed_case/certificates/client-key.pem.rollback-$delayed_stamp" \
    && ! -e "$delayed_case/certificates/config-$delayed_stamp" \
    && ! -d "$delayed_case/upload" ]] || fail "$delayed_probe delay left temporary rotation files"
  echo "runtime-certificate-rotation-test: transient $delayed_probe failure retried successfully"
done

for failed_probe in health ready; do
  failed_case="$test_root/probe-exhausted-$failed_probe"
  failed_stamp='20260905T090909Z'
  prepare_case "$failed_case" "$failed_stamp"
  health_failures=0
  ready_failures=0
  if [[ "$failed_probe" == health ]]; then health_failures=15; else ready_failures=15; fi
  actual_status=0
  STUB_HEALTH_FAILURES="$health_failures" STUB_READY_FAILURES="$ready_failures" \
    STUB_PROBE_FAILURE_STATUS=28 \
    run_activation "$failed_case" "$failed_stamp" false \
      >"$failed_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" == 28 ]] || fail "$failed_probe exhaustion lost the last probe exit status"
  [[ "$(grep -c "^$failed_probe$" "$failed_case/curl.log")" == 15 \
    && "$(grep -c '^1$' "$failed_case/sleep.log")" == 14 ]] \
    || fail "$failed_probe exhaustion did not stop after 15 attempts with 14 waits"
  if grep -v -- '--connect-timeout 2 --max-time 5' "$failed_case/curl-arguments.log"; then
    fail "$failed_probe probe did not set connection and request timeouts"
  fi
  assert_single_recovery "$failed_case" "$failed_stamp" active
  echo "runtime-certificate-rotation-test: exhausted $failed_probe retries recovered exactly once"
done

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

for aws_fail_at in active-sts candidate-sts active-s3api; do
  aws_case="$test_root/aws-$aws_fail_at"
  aws_stamp='20260905T060606Z'
  prepare_case "$aws_case" "$aws_stamp"
  actual_status=0
  STUB_AWS_FAIL_AT="$aws_fail_at" run_activation "$aws_case" "$aws_stamp" false \
    >"$aws_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" == 42 ]] || fail "$aws_fail_at failure lost the AWS exit status"
  assert_single_recovery "$aws_case" "$aws_stamp" "${aws_fail_at%%-*}"
  echo "runtime-certificate-rotation-test: $aws_fail_at failure recovered exactly once"
done

for invalid_pem in key cert; do
  pem_case="$test_root/invalid-$invalid_pem"
  pem_stamp='20260905T070707Z'
  prepare_case "$pem_case" "$pem_stamp"
  printf 'invalid PEM\n' >"$pem_case/upload/vps-client-$invalid_pem-$pem_stamp.pem"
  actual_status=0
  run_activation "$pem_case" "$pem_stamp" false \
    >"$pem_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" != 0 ]] || fail "invalid $invalid_pem was accepted"
  assert_single_recovery "$pem_case" "$pem_stamp" preflight
  echo "runtime-certificate-rotation-test: invalid $invalid_pem cleaned up exactly once"
done

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
  for signal_point in service-stopped certificate-installed probes probe-wait; do
    signal_case="$test_root/signal-$signal_name-$signal_point"
    signal_stamp='20260905T030303Z'
    prepare_case "$signal_case" "$signal_stamp"
    actual_status=0
    STUB_SIGNAL="$signal_name" STUB_SIGNAL_POINT="$signal_point" STUB_HEALTH_FAILURES=1 \
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

prepare_local_rotation_case() {
  local case_dir="$1"

  mkdir -p "$case_dir/stubs" "$case_dir/uploaded"
  : >"$case_dir/transport.log"
  create_ca "$case_dir"
  create_client_certificate "$case_dir" seed
  cat >"$case_dir/stubs/hostname" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "${STUB_HOSTNAME-test-management}"
exit "${STUB_HOSTNAME_STATUS:-0}"
EOF
  cat >"$case_dir/stubs/ssh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
port=''
if [[ "${1:-}" == '-p' ]]; then
  port="$2"
  shift 2
fi
[[ "$port" == "${STUB_EXPECTED_SSH_PORT:-}" ]]
printf 'ssh %s\n' "$port" >>"$STUB_TRANSPORT_LOG"
if [[ "${1:-}" == '-tt' ]]; then
  if [[ -n "${STUB_LOCAL_SIGNAL:-}" ]]; then
    kill -s "$STUB_LOCAL_SIGNAL" "$PPID"
  fi
  exit "${STUB_SSH_ACTIVATION_STATUS:-0}"
fi
exit 0
EOF
  cat >"$case_dir/stubs/scp" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
port=''
if [[ "${1:-}" == '-P' ]]; then
  port="$2"
  shift 2
fi
[[ "$port" == "${STUB_EXPECTED_SSH_PORT:-}" ]]
printf 'scp %s\n' "$port" >>"$STUB_TRANSPORT_LOG"
cp "$1" "$STUB_UPLOADED_DIR/cert.pem"
cp "$2" "$STUB_UPLOADED_DIR/key.pem"
EOF
  chmod +x "$case_dir/stubs/hostname" "$case_dir/stubs/ssh" "$case_dir/stubs/scp"
}

run_local_rotation() {
  local case_dir="$1"

  PATH="$case_dir/stubs:$PATH" \
    OKAWAK_BLOG_PKI_DIR="$case_dir" \
    STUB_UPLOADED_DIR="$case_dir/uploaded" \
    STUB_TRANSPORT_LOG="$case_dir/transport.log" \
    bash "$rotation_script" test-vps
}

test_subject_cn() {
  local case_name="$1"
  local expected_cn="$2"
  local expected_days="${3:-90}"
  local case_dir="$test_root/cn-$case_name"

  prepare_local_rotation_case "$case_dir"
  local actual_status=0
  run_local_rotation "$case_dir" >"$case_dir/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" == "${STUB_ROTATION_STATUS:-0}" ]] \
    || fail "$case_name rotation: unexpected exit status $actual_status"

  openssl verify -purpose sslclient -CAfile "$case_dir/ca-cert.pem" \
    "$case_dir/uploaded/cert.pem" >/dev/null
  openssl x509 -checkend 604800 -noout -in "$case_dir/uploaded/cert.pem" >/dev/null \
    || fail "$case_name certificate would fail the VPS seven-day validity check"
  openssl x509 -checkend "$((expected_days * 86400 - 60))" -noout \
    -in "$case_dir/uploaded/cert.pem" >/dev/null \
    || fail "$case_name certificate expires earlier than the configured validity"
  if openssl x509 -checkend "$((expected_days * 86400 + 60))" -noout \
    -in "$case_dir/uploaded/cert.pem" >/dev/null; then
    fail "$case_name certificate expires later than the configured validity"
  fi
  local subject
  subject="$(openssl x509 -in "$case_dir/uploaded/cert.pem" \
    -noout -subject -nameopt sep_multiline,sname,utf8)"
  [[ "$(printf '%s\n' "$subject" | sed -n 's/^ *CN=//p')" == "$expected_cn" ]] \
    || fail "issued certificate CN does not match $expected_cn: $subject"
  [[ ! -d "$case_dir/.certificate-rotation.lock" ]] || fail "rotation lock was retained"
  local expected_ssh_calls=2
  if [[ "$actual_status" != 0 ]]; then expected_ssh_calls=3; fi
  [[ "$(grep -Fxc "ssh ${STUB_EXPECTED_SSH_PORT:-}" "$case_dir/transport.log")" == "$expected_ssh_calls" \
    && "$(grep -Fxc "scp ${STUB_EXPECTED_SSH_PORT:-}" "$case_dir/transport.log")" == 1 ]] \
    || fail "$case_name did not use the same SSH port for staging, activation and cleanup"
  echo "runtime-certificate-rotation-test: $case_name CN matches the issued certificate"
}

unset OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN
unset OKAWAK_BLOG_VPS_SSH_PORT
unset OKAWAK_BLOG_CERTIFICATE_DAYS
export OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST=test-management

for guard_case in unregistered empty-registration wrong-host empty-hostname hostname-failure invalid-registration; do
  host_case="$test_root/host-$guard_case"
  prepare_local_rotation_case "$host_case"
  cp "$host_case/ca-cert.srl" "$host_case/ca-cert.srl.before"
  actual_status=0
  (
    case "$guard_case" in
      unregistered) unset OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST ;;
      empty-registration) export OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST='' ;;
      wrong-host) export STUB_HOSTNAME=test-vps ;;
      empty-hostname) export STUB_HOSTNAME='' ;;
      # Even plausible output must not be accepted after a failed hostname query.
      hostname-failure) export STUB_HOSTNAME_STATUS=42 ;;
      invalid-registration) export OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST='not a hostname' ;;
    esac
    run_local_rotation "$host_case"
  ) >"$host_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" == 1 ]] || fail "$guard_case management-host guard did not reject execution"
  grep -Fq 'certificate-rotation: management-host guard:' "$host_case/output.log" \
    || fail "$guard_case did not explain the management-host restriction"
  cmp -s "$host_case/ca-cert.srl.before" "$host_case/ca-cert.srl" \
    || fail "$guard_case modified the CA serial"
  [[ ! -s "$host_case/transport.log" && ! -d "$host_case/.certificate-rotation.lock" ]] \
    || fail "$guard_case contacted the VPS or retained a lock"
  if compgen -G "$host_case/vps-client-*" >/dev/null; then
    fail "$guard_case created local issuance files"
  fi
done
echo 'runtime-certificate-rotation-test: management-host guard rejects before issuance or transfer'

help_case="$test_root/host-help"
prepare_local_rotation_case "$help_case"
env -u OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST PATH="$help_case/stubs:$PATH" STUB_HOSTNAME=test-vps \
  bash "$rotation_script" --help >"$help_case/output.log" 2>&1 \
  || fail 'help was blocked on an unregistered host'
grep -Fq 'Usage: mise run rotate-runtime-certificate' "$help_case/output.log" \
  || fail 'help did not display usage'

for invalid_days in 1 2 3 4 5 6 7 0 -1 abc 1.5 +8 08 '8 days'; do
  days_case="$test_root/invalid-days-$invalid_days"
  prepare_local_rotation_case "$days_case"
  cp "$days_case/ca-cert.srl" "$days_case/ca-cert.srl.before"
  actual_status=0
  OKAWAK_BLOG_CERTIFICATE_DAYS="$invalid_days" \
    run_local_rotation "$days_case" >"$days_case/output.log" 2>&1 || actual_status=$?
  [[ "$actual_status" == 1 ]] || fail "invalid certificate validity was accepted: $invalid_days"
  grep -Fq 'OKAWAK_BLOG_CERTIFICATE_DAYS must be an integer of at least 8' "$days_case/output.log" \
    || fail "invalid certificate validity was not rejected before issuance: $invalid_days"
  cmp -s "$days_case/ca-cert.srl.before" "$days_case/ca-cert.srl" \
    || fail "invalid certificate validity modified the CA serial"
  [[ ! -s "$days_case/transport.log" && ! -d "$days_case/.certificate-rotation.lock" ]] \
    || fail "invalid certificate validity contacted the VPS or retained a lock"
  if compgen -G "$days_case/vps-client-*" >/dev/null; then
    fail "invalid certificate validity created local issuance files"
  fi
done
echo 'runtime-certificate-rotation-test: invalid validity rejected without issuance or transfer'

test_subject_cn default okawak-blog-vps
OKAWAK_BLOG_CERTIFICATE_DAYS='' test_subject_cn empty-days okawak-blog-vps
OKAWAK_BLOG_CERTIFICATE_DAYS=8 test_subject_cn minimum-days okawak-blog-vps 8
OKAWAK_BLOG_CERTIFICATE_DAYS=9 test_subject_cn nine-days okawak-blog-vps 9
OKAWAK_BLOG_CERTIFICATE_DAYS=10 test_subject_cn custom-days okawak-blog-vps 10
OKAWAK_BLOG_VPS_SSH_PORT=60022 STUB_EXPECTED_SSH_PORT=60022 \
  test_subject_cn ssh-port okawak-blog-vps
OKAWAK_BLOG_VPS_SSH_PORT=2222 STUB_EXPECTED_SSH_PORT=2222 \
  STUB_SSH_ACTIVATION_STATUS=42 STUB_ROTATION_STATUS=42 \
  test_subject_cn ssh-port-failure okawak-blog-vps
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
  OKAWAK_BLOG_VPS_SSH_PORT=60022 STUB_EXPECTED_SSH_PORT=60022 \
    STUB_LOCAL_SIGNAL="$signal_name" STUB_ROTATION_STATUS="$expected_status" \
    test_subject_cn "local-signal-$signal_name" okawak-blog-vps
done

for invalid_port in 0 65536 -1 abc '22 -oProxyCommand=unexpected' 99999999999999999999; do
  if OKAWAK_BLOG_VPS_SSH_PORT="$invalid_port" OKAWAK_BLOG_PKI_DIR="$test_root/not-created" \
    bash "$rotation_script" test-vps >"$test_root/invalid-port.log" 2>&1; then
    fail "invalid SSH port was accepted: $invalid_port"
  fi
  grep -q 'OKAWAK_BLOG_VPS_SSH_PORT must be' "$test_root/invalid-port.log" \
    || fail 'invalid SSH port was not rejected before accessing the CA'
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
