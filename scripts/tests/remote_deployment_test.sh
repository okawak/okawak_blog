#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
deployment_script="$repo_root/scripts/vps.sh"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT

fail() {
  echo "remote-deployment-test: $*" >&2
  exit 1
}

mkdir -p "$test_root/bin"
cat >"$test_root/bin/ssh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" >"$STUB_SSH_LOG"
[[ "${STUB_SSH_STATUS:-0}" == 0 ]] || exit "$STUB_SSH_STATUS"
[[ "$1" == -tt ]]
shift
shift # SSH target
[[ $# == 1 ]]
# Exercise shell quoting, but never load the test runner's real login profile.
bash() {
  [[ "$1" == -lc && $# == 2 ]]
  /bin/bash --noprofile --norc -c "$2"
}
cd() {
  [[ "$1" == "${STUB_EXPECTED_REPO_DIR:-/opt/okawak_blog}" ]]
  printf 'cd %s\n' "$1" >>"$STUB_REMOTE_LOG"
  return "${STUB_CD_STATUS:-0}"
}
export -f bash cd
/bin/bash --noprofile --norc -c "$1"
EOF
cat >"$test_root/bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'git %s\n' "$*" >>"$STUB_REMOTE_LOG"
case "$*" in
  'branch --show-current') printf '%s\n' "${STUB_BRANCH-main}" ;;
  'status --porcelain') printf '%s' "${STUB_DIRTY:-}"; exit "${STUB_STATUS_CODE:-0}" ;;
  'fetch origin main') exit "${STUB_FETCH_STATUS:-0}" ;;
  'merge-base --is-ancestor HEAD FETCH_HEAD') exit "${STUB_AHEAD_STATUS:-0}" ;;
  *) exit 98 ;;
esac
EOF
cat >"$test_root/bin/mise" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'mise %s\n' "$*" >>"$STUB_REMOTE_LOG"
[[ "$*" == 'run production-deploy' ]]
exit "${STUB_DEPLOY_STATUS:-0}"
EOF
cat >"$test_root/bin/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'sudo %s\n' "$*" >>"$STUB_REMOTE_LOG"
exit "${STUB_OPS_STATUS:-0}"
EOF
chmod +x "$test_root/bin/"*
export PATH="$test_root/bin:$PATH"
export STUB_SSH_LOG="$test_root/ssh.log"
export STUB_REMOTE_LOG="$test_root/remote.log"
unset OKAWAK_BLOG_VPS_SSH_TARGET OKAWAK_BLOG_VPS_SSH_PORT OKAWAK_BLOG_VPS_REPO_DIR

run_case() {
  local expected_status="$1"
  shift
  : >"$STUB_SSH_LOG"
  : >"$STUB_REMOTE_LOG"
  local status=0
  "$@" >"$test_root/output.log" 2>&1 || status=$?
  [[ "$status" == "$expected_status" ]] \
    || fail "expected status $expected_status, got $status: $(cat "$test_root/output.log")"
}

for script in "$deployment_script" "$repo_root/scripts/rotate_runtime_certificate.sh"; do
  set -- bash "$script"
  [[ "$script" != "$deployment_script" ]] || set -- "$@" deploy
  run_case 1 "$@"
  grep -q 'OKAWAK_BLOG_VPS_SSH_TARGET' "$test_root/output.log" || fail "missing target setup guidance"
  [[ ! -s "$STUB_SSH_LOG" && ! -s "$STUB_REMOTE_LOG" ]] || fail "missing target contacted VPS"
  run_case 1 env OKAWAK_BLOG_VPS_SSH_TARGET='' "$@"
  grep -q 'OKAWAK_BLOG_VPS_SSH_TARGET' "$test_root/output.log" || fail "empty target setup guidance missing"
  [[ ! -s "$STUB_SSH_LOG" && ! -s "$STUB_REMOTE_LOG" ]] || fail "empty target contacted VPS"
done
export OKAWAK_BLOG_VPS_SSH_TARGET=test-vps

run_case 1 bash "$deployment_script" deploy
grep -q 'OKAWAK_BLOG_VPS_REPO_DIR' "$test_root/output.log" || fail "missing directory setup guidance"
[[ ! -s "$STUB_SSH_LOG" && ! -s "$STUB_REMOTE_LOG" ]] || fail "missing VPS directory contacted SSH"
run_case 0 bash "$deployment_script" deploy --help
[[ ! -s "$STUB_SSH_LOG" ]] || fail "help without directory contacted SSH"
export OKAWAK_BLOG_VPS_REPO_DIR=/opt/okawak_blog

run_case 0 bash "$deployment_script" deploy
[[ "$(head -2 "$STUB_SSH_LOG")" == $'-tt\ntest-vps' ]] || fail "configured SSH alias/TTY not used"
[[ "$(cat "$STUB_REMOTE_LOG")" == $'cd /opt/okawak_blog\ngit branch --show-current\ngit status --porcelain\ngit fetch origin main\ngit merge-base --is-ancestor HEAD FETCH_HEAD\nmise run production-deploy' ]] \
  || fail "remote preflight/deploy order is incorrect"

run_case 0 env OKAWAK_BLOG_VPS_REPO_DIR=/srv/apps/okawak_blog STUB_EXPECTED_REPO_DIR=/srv/apps/okawak_blog \
  bash "$deployment_script" deploy
[[ "$(head -1 "$STUB_REMOTE_LOG")" == 'cd /srv/apps/okawak_blog' ]] || fail "configured VPS directory not used"
for invalid_dir in '' relative/path '~/okawak_blog' / ' /srv/blog' '/srv/blog;echo bad' '/srv/$(echo bad)' '/srv/blog%h' '/srv/blog name'; do
  run_case 1 env OKAWAK_BLOG_VPS_REPO_DIR="$invalid_dir" bash "$deployment_script" deploy
  grep -q 'OKAWAK_BLOG_VPS_REPO_DIR' "$test_root/output.log" || fail "missing directory setup guidance"
  [[ ! -s "$STUB_SSH_LOG" ]] || fail "invalid VPS directory contacted SSH"
done
run_case 1 env STUB_CD_STATUS=1 bash "$deployment_script" deploy
! grep -q '^git \|^mise ' "$STUB_REMOTE_LOG" || fail "missing VPS directory advanced deployment"

run_case 0 env OKAWAK_BLOG_VPS_SSH_TARGET=other-host OKAWAK_BLOG_VPS_SSH_PORT=60022 \
  bash "$deployment_script" deploy user@vps
[[ "$(head -2 "$STUB_SSH_LOG")" == $'-tt\nuser@vps' ]] || fail "SSH argument precedence/config not applied"
run_case 0 env OKAWAK_BLOG_VPS_SSH_TARGET=other-host bash "$deployment_script" deploy
[[ "$(sed -n '2p' "$STUB_SSH_LOG")" == other-host ]] || fail "environment target not used"

run_case 0 bash "$deployment_script" deploy --help
[[ ! -s "$STUB_SSH_LOG" ]] || fail "help contacted SSH"
for invalid_target in -oProxyCommand=bad 'host;echo bad' 'host name'; do
  run_case 1 bash "$deployment_script" deploy "$invalid_target"
  [[ ! -s "$STUB_SSH_LOG" ]] || fail "invalid target contacted SSH"
done
run_case 1 bash "$deployment_script" deploy test-vps unexpected-argument
[[ ! -s "$STUB_SSH_LOG" ]] || fail "extra arguments contacted SSH"

for branch in topic ''; do
  run_case 1 env STUB_BRANCH="$branch" bash "$deployment_script" deploy
  ! grep -q '^git fetch\|^mise ' "$STUB_REMOTE_LOG" || fail "wrong/detached branch advanced deployment"
done
run_case 1 env STUB_DIRTY=' M mise.toml' bash "$deployment_script" deploy
! grep -q '^git fetch\|^mise ' "$STUB_REMOTE_LOG" || fail "dirty checkout advanced deployment"
run_case 2 env STUB_STATUS_CODE=2 bash "$deployment_script" deploy
! grep -q '^git fetch\|^mise ' "$STUB_REMOTE_LOG" || fail "status failure advanced deployment"
run_case 128 env STUB_FETCH_STATUS=128 bash "$deployment_script" deploy
! grep -q '^mise ' "$STUB_REMOTE_LOG" || fail "fetch failure deployed"
run_case 1 env STUB_AHEAD_STATUS=1 bash "$deployment_script" deploy
! grep -q '^mise ' "$STUB_REMOTE_LOG" || fail "unmerged VPS commit deployed"
run_case 42 env STUB_DEPLOY_STATUS=42 bash "$deployment_script" deploy
run_case 255 env STUB_SSH_STATUS=255 bash "$deployment_script" deploy
[[ ! -s "$STUB_REMOTE_LOG" ]] || fail "SSH failure ran a remote command"

for operation in status logs logs-recent restart; do
  run_case 0 bash "$deployment_script" "$operation"
  case "$operation" in
    status) expected='sudo systemctl status --no-pager okawak_blog' ;;
    logs) expected='sudo journalctl -u okawak_blog --no-pager -f' ;;
    logs-recent) expected='sudo journalctl -u okawak_blog --no-pager --lines=50' ;;
    restart) expected='sudo systemctl restart okawak_blog' ;;
  esac
  [[ "$(cat "$STUB_REMOTE_LOG")" == "$expected" ]] || fail "incorrect remote operation: $operation"
done
run_case 3 env STUB_OPS_STATUS=3 bash "$deployment_script" status
run_case 1 bash "$deployment_script" unsupported
[[ ! -s "$STUB_SSH_LOG" ]] || fail "unsupported operation contacted SSH"

echo 'remote-deployment-test: all checks passed' 
