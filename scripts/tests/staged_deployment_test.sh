#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
activation_script="$repo_root/scripts/activate_staged_deployment.sh"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT

fail() {
  echo "staged-deployment-test: $*" >&2
  exit 1
}

write_command_stubs() {
  local stub_dir="$1"

  mkdir -p "$stub_dir"
  cat >"$stub_dir/sudo" <<'EOF'
#!/usr/bin/env bash
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
  cat >"$stub_dir/mv" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${STUB_UNIT_RESTORE_FAIL:-false}" == true && "$1" == -f \
  && "$2" == "$STUB_SYSTEMD_DIR/okawak_blog.service.rollback" ]]; then
  exit 1
fi
exec /bin/mv "$@"
EOF
  cat >"$stub_dir/chown" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat >"$stub_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "is-active" ]]; then
  [[ "${STUB_SERVICE_ACTIVE:-false}" == "true" ]]
  exit
fi
printf '%s\n' "$*" >>"$STUB_SYSTEMCTL_LOG"
if [[ "$1" == start ]]; then
  unit="$STUB_SYSTEMD_DIR/okawak_blog.service"
  executable="$(sed -n 's/^ExecStart=//p' "$unit")"
  [[ -x "$executable" ]] || exit 1
  printf 'started %s\n' "$executable" >>"$STUB_SYSTEMCTL_LOG"
  if [[ "${STUB_START_FAIL_ONCE:-false}" == true && ! -e "$STUB_SYSTEMD_DIR/start-failed" ]]; then
    touch "$STUB_SYSTEMD_DIR/start-failed"
    exit 1
  fi
fi
EOF
  cat >"$stub_dir/curl" <<'EOF'
#!/usr/bin/env bash
printf 'curl %s\n' "$*" >>"$STUB_PROBE_LOG"
if [[ "${STUB_CURL_FAIL:-false}" == "true" ]]; then
  exit 22
fi
exit 0
EOF
  cat >"$stub_dir/sleep" <<'EOF'
#!/usr/bin/env bash
printf 'sleep %s\n' "$*" >>"$STUB_PROBE_LOG"
exit 0
EOF
  chmod +x "$stub_dir"/*
}

write_bundle() {
  local bundle_dir="$1"

  mkdir -p "$bundle_dir"
  cat >"$bundle_dir/manifest.toml" <<'EOF'
version = 1

[[assets]]
id = 1
file = "tailwind-new.css"
hash = "css"
content_type = "text/css"

[[assets]]
id = 2
file = "topcoat-runtime-new.js"
hash = "topcoat-runtime"
content_type = "text/javascript"

[[assets]]
id = 3
file = "favicon-new.ico"
hash = "favicon"
content_type = "image/x-icon"
EOF
  printf 'new css\n' >"$bundle_dir/tailwind-new.css"
  printf 'new js\n' >"$bundle_dir/topcoat-runtime-new.js"
  printf 'new icon\n' >"$bundle_dir/favicon-new.ico"
}

prepare_case() {
  local case_dir="$1"

  mkdir -p \
    "$case_dir/bin/assets" \
    "$case_dir/systemd" \
    "$case_dir/target/release"
  write_command_stubs "$case_dir/stubs"
  write_bundle "$case_dir/target/assets-staged"
  cp "$repo_root/service/okawak_blog.service" "$case_dir/service.service"
  sed \
    -e "s|^WorkingDirectory=.*|WorkingDirectory=$case_dir|" \
    -e "s|^ExecStart=.*|ExecStart=$case_dir/bin/okawak_blog|" \
    "$case_dir/service.service" >"$case_dir/systemd/okawak_blog.service"
  cp "$case_dir/systemd/okawak_blog.service" "$case_dir/previous.service"
  printf 'old binary\n' >"$case_dir/bin/okawak_blog"
  printf 'old asset\n' >"$case_dir/bin/assets/old.css"
  printf '#!/usr/bin/env bash\necho new binary\n' >"$case_dir/target/release/server"
  chmod +x "$case_dir/bin/okawak_blog" "$case_dir/target/release/server"
  : >"$case_dir/systemctl.log"
  : >"$case_dir/probe.log"
}

run_activation() {
  local case_dir="$1"
  local curl_fail="$2"

  PATH="$case_dir/stubs:$PATH" \
    REPO_ROOT="$case_dir" \
    SERVICE_FILE="$case_dir/service.service" \
    SYSTEMD_UNIT_DIR="$case_dir/systemd" \
    TARGET_BIN="$case_dir/target/release/server" \
    BIN_DIR="$case_dir/bin" \
    DEPLOY_STAGED_ASSETS="$case_dir/target/assets-staged" \
    DEPLOY_PROBE_ATTEMPTS=1 \
    STUB_SERVICE_ACTIVE="${STUB_SERVICE_ACTIVE:-true}" \
    STUB_CURL_FAIL="$curl_fail" \
    STUB_PROBE_LOG="$case_dir/probe.log" \
    STUB_SYSTEMCTL_LOG="$case_dir/systemctl.log" \
    STUB_SYSTEMD_DIR="$case_dir/systemd" \
    bash "$activation_script"
}

success_case="$test_root/success"
prepare_case "$success_case"
run_activation "$success_case" false
[[ "$(head -1 "$success_case/probe.log")" == 'sleep 1' ]] \
  || fail "activation did not wait before the first health probe"
grep -Fxq "WorkingDirectory=$success_case" "$success_case/systemd/okawak_blog.service" \
  || fail "systemd working directory does not match the deployment directory"
grep -Fxq "ExecStart=$success_case/bin/okawak_blog" "$success_case/systemd/okawak_blog.service" \
  || fail "systemd executable does not match the installed binary"
grep -Fxq 'ProtectHome=true' "$success_case/systemd/okawak_blog.service" \
  || fail "systemd hardening was lost"
cmp -s "$repo_root/service/okawak_blog.service" "$success_case/service.service" \
  || fail "deployment modified the source service unit"
cmp -s "$success_case/target/release/server" "$success_case/bin/okawak_blog" \
  || fail "successful activation did not install the new binary"
[[ -f "$success_case/bin/assets/tailwind-new.css" ]] \
  || fail "successful activation did not install the new assets"
[[ ! -e "$success_case/bin/assets/old.css" ]] \
  || fail "successful activation retained the old assets"
[[ ! -e "$success_case/bin/assets.rollback" ]] \
  || fail "successful activation retained rollback assets"
[[ ! -e "$success_case/bin/assets.failed" ]] \
  || fail "successful activation created failed assets"

[[ ! -e "$success_case/systemd/okawak_blog.service.rollback" ]] \
  || fail "successful activation retained the unit backup"

rollback_case="$test_root/rollback"
prepare_case "$rollback_case"
if run_activation "$rollback_case" true; then
  fail "failed probes unexpectedly completed activation"
fi
cmp -s "$rollback_case/previous.service" "$rollback_case/systemd/okawak_blog.service" \
  || fail "probe failure did not restore the previous unit"
grep -qx 'old binary' "$rollback_case/bin/okawak_blog" \
  || fail "probe failure did not restore the old binary"
[[ -f "$rollback_case/bin/assets/old.css" ]] \
  || fail "probe failure did not restore the old assets"
[[ -f "$rollback_case/bin/assets.failed/tailwind-new.css" ]] \
  || fail "probe failure did not preserve the failed assets"
grep -qx 'start okawak_blog.service' "$rollback_case/systemctl.log" \
  || fail "probe failure did not restart the previously active service"

# A new checkout has no previous binary; the running service still points to the old checkout.
for failure_point in probes start unit-restore; do
  moved_case="$test_root/moved-$failure_point"
  prepare_case "$moved_case"
  mkdir -p "$moved_case/old-checkout/bin"
  mv "$moved_case/bin/okawak_blog" "$moved_case/old-checkout/bin/okawak_blog"
  mv "$moved_case/bin/assets" "$moved_case/old-checkout/bin/assets"
  sed \
    -e "s|^WorkingDirectory=.*|WorkingDirectory=$moved_case/old-checkout|" \
    -e "s|^ExecStart=.*|ExecStart=$moved_case/old-checkout/bin/okawak_blog|" \
    "$moved_case/service.service" >"$moved_case/systemd/okawak_blog.service"
  cp "$moved_case/systemd/okawak_blog.service" "$moved_case/previous.service"
  curl_fail=true
  start_fail=false
  restore_fail=false
  if [[ "$failure_point" == start ]]; then curl_fail=false; start_fail=true; fi
  if [[ "$failure_point" == unit-restore ]]; then restore_fail=true; fi
  if STUB_START_FAIL_ONCE="$start_fail" STUB_UNIT_RESTORE_FAIL="$restore_fail" run_activation "$moved_case" "$curl_fail"; then
    fail "$failure_point failure unexpectedly completed relocation"
  fi
  if [[ "$restore_fail" == true ]]; then
    cmp -s "$moved_case/previous.service" "$moved_case/systemd/okawak_blog.service.rollback" \
      || fail "unit recovery failure lost the unit backup"
    [[ "$(tail -1 "$moved_case/systemctl.log")" == 'stop okawak_blog.service' ]] \
      || fail "unit recovery failure restarted the service with the wrong unit"
    continue
  fi
  cmp -s "$moved_case/previous.service" "$moved_case/systemd/okawak_blog.service" \
    || fail "$failure_point failure did not restore the previous systemd unit"
  [[ "$(tail -1 "$moved_case/systemctl.log")" == "started $moved_case/old-checkout/bin/okawak_blog" ]] \
    || fail "$failure_point failure did not restart the old checkout"
  [[ ! -e "$moved_case/bin/okawak_blog" ]] || fail "relocation retained the failed new binary"
  [[ -f "$moved_case/old-checkout/bin/assets/old.css" ]] || fail "relocation modified the old assets"
  [[ ! -e "$moved_case/systemd/okawak_blog.service.rollback" ]] || fail "restored unit left a rollback file"
done

fresh_case="$test_root/fresh"
prepare_case "$fresh_case"
rm "$fresh_case/systemd/okawak_blog.service"
if STUB_SERVICE_ACTIVE=false run_activation "$fresh_case" true; then
  fail "failed first deployment unexpectedly completed"
fi
[[ ! -e "$fresh_case/systemd/okawak_blog.service" ]] || fail "failed first deployment retained the new unit"
[[ "$(tail -1 "$fresh_case/systemctl.log")" == daemon-reload ]] \
  || fail "failed first deployment restarted a previously inactive service"

backup_case="$test_root/existing-unit-backup"
prepare_case "$backup_case"
cp "$backup_case/previous.service" "$backup_case/systemd/okawak_blog.service.rollback"
if run_activation "$backup_case" false; then
  fail "existing unit backup was accepted"
fi
[[ ! -s "$backup_case/systemctl.log" ]] || fail "unit backup conflict changed service state"
cmp -s "$backup_case/previous.service" "$backup_case/systemd/okawak_blog.service.rollback" \
  || fail "unit backup conflict overwrote the backup"

wasm_case="$test_root/wasm"
prepare_case "$wasm_case"
printf 'wasm\n' >"$wasm_case/target/assets-staged/client.wasm"
cat >>"$wasm_case/target/assets-staged/manifest.toml" <<'EOF'

[[assets]]
id = 4
file = "client.wasm"
hash = "wasm"
content_type = "application/wasm"
EOF
if run_activation "$wasm_case" false; then
  fail "WebAssembly bundle unexpectedly passed deployment validation"
fi
grep -qx 'old binary' "$wasm_case/bin/okawak_blog" \
  || fail "preflight failure changed the installed binary"
[[ -f "$wasm_case/bin/assets/old.css" ]] \
  || fail "preflight failure changed the installed assets"
[[ -d "$wasm_case/target/assets-staged" ]] \
  || fail "preflight failure consumed the staged assets"

echo "staged-deployment-test: all cases passed"
