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
EOF
  cat >"$stub_dir/curl" <<'EOF'
#!/usr/bin/env bash
if [[ "${STUB_CURL_FAIL:-false}" == "true" ]]; then
  exit 22
fi
exit 0
EOF
  cat >"$stub_dir/sleep" <<'EOF'
#!/usr/bin/env bash
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
file = "navigation-new.js"
hash = "navigation"
content_type = "text/javascript"

[[assets]]
id = 3
file = "favicon-new.ico"
hash = "favicon"
content_type = "image/x-icon"
EOF
  printf 'new css\n' >"$bundle_dir/tailwind-new.css"
  printf 'new js\n' >"$bundle_dir/navigation-new.js"
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
  printf 'unit\n' >"$case_dir/service.service"
  printf 'old binary\n' >"$case_dir/bin/okawak_blog"
  printf 'old asset\n' >"$case_dir/bin/assets/old.css"
  printf '#!/usr/bin/env bash\necho new binary\n' >"$case_dir/target/release/topcoat-server"
  chmod +x "$case_dir/bin/okawak_blog" "$case_dir/target/release/topcoat-server"
  : >"$case_dir/systemctl.log"
}

run_activation() {
  local case_dir="$1"
  local curl_fail="$2"

  PATH="$case_dir/stubs:$PATH" \
    REPO_ROOT="$case_dir" \
    SERVICE_FILE="$case_dir/service.service" \
    SYSTEMD_UNIT_DIR="$case_dir/systemd" \
    TARGET_BIN="$case_dir/target/release/topcoat-server" \
    BIN_DIR="$case_dir/bin" \
    DEPLOY_STAGED_ASSETS="$case_dir/target/assets-staged" \
    DEPLOY_PROBE_ATTEMPTS=1 \
    STUB_SERVICE_ACTIVE=true \
    STUB_CURL_FAIL="$curl_fail" \
    STUB_SYSTEMCTL_LOG="$case_dir/systemctl.log" \
    bash "$activation_script"
}

success_case="$test_root/success"
prepare_case "$success_case"
run_activation "$success_case" false
cmp -s "$success_case/target/release/topcoat-server" "$success_case/bin/okawak_blog" \
  || fail "successful activation did not install the new binary"
[[ -f "$success_case/bin/assets/tailwind-new.css" ]] \
  || fail "successful activation did not install the new assets"
[[ ! -e "$success_case/bin/assets/old.css" ]] \
  || fail "successful activation retained the old assets"
[[ ! -e "$success_case/bin/assets.rollback" ]] \
  || fail "successful activation retained rollback assets"
[[ ! -e "$success_case/bin/assets.failed" ]] \
  || fail "successful activation created failed assets"

rollback_case="$test_root/rollback"
prepare_case "$rollback_case"
if run_activation "$rollback_case" true; then
  fail "failed probes unexpectedly completed activation"
fi
grep -qx 'old binary' "$rollback_case/bin/okawak_blog" \
  || fail "probe failure did not restore the old binary"
[[ -f "$rollback_case/bin/assets/old.css" ]] \
  || fail "probe failure did not restore the old assets"
[[ -f "$rollback_case/bin/assets.failed/tailwind-new.css" ]] \
  || fail "probe failure did not preserve the failed assets"
grep -qx 'start okawak_blog.service' "$rollback_case/systemctl.log" \
  || fail "probe failure did not restart the previously active service"

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
