#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
sync_script="$repo_root/scripts/sync_obsidian.sh"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT

fail() {
  echo "sync-obsidian-test: $*" >&2
  exit 1
}

mkdir -p "$test_root/bin"
cat >"$test_root/bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'git %s\n' "$*" >>"$STUB_GIT_LOG"
case "$*" in
  'submodule update --init --recursive obsidian')
    mkdir -p "$STUB_REPO/obsidian"
    touch "$STUB_REPO/obsidian/.git"
    ;;
  '-C obsidian status --porcelain') printf '%s' "${STUB_DIRTY:-}" ;;
  'submodule update --remote --checkout --recursive obsidian') ;;
  *) exit 98 ;;
esac
EOF
chmod +x "$test_root/bin/git"
export PATH="$test_root/bin:$PATH"

run_sync() {
  local repo="$1"
  local expected_status="$2"
  local status=0
  export STUB_REPO="$repo"
  export STUB_GIT_LOG="$repo/git.log"
  : >"$STUB_GIT_LOG"
  (cd "$repo" && bash "$sync_script") >"$repo/output.log" 2>&1 || status=$?
  [[ "$status" == "$expected_status" ]] \
    || fail "expected status $expected_status, got $status: $(cat "$repo/output.log")"
}

legacy_repo="$test_root/legacy"
mkdir -p "$legacy_repo/crates/publish/obsidian"
touch "$legacy_repo/crates/publish/obsidian/.git"
run_sync "$legacy_repo" 1
grep -Fq 'crates/publish/obsidian' "$legacy_repo/output.log" \
  || fail "legacy path guidance missing"
[[ ! -s "$legacy_repo/git.log" ]] || fail "legacy checkout reached git synchronization"

new_repo="$test_root/new"
mkdir -p "$new_repo"
run_sync "$new_repo" 0
[[ "$(cat "$new_repo/git.log")" == $'git submodule update --init --recursive obsidian\ngit -C obsidian status --porcelain\ngit submodule update --remote --checkout --recursive obsidian' ]] \
  || fail "new checkout synchronization order is incorrect"

dirty_repo="$test_root/dirty"
mkdir -p "$dirty_repo/obsidian"
touch "$dirty_repo/obsidian/.git"
STUB_DIRTY=' M Publish/note.md' run_sync "$dirty_repo" 1
grep -Fq 'uncommitted changes' "$dirty_repo/output.log" \
  || fail "dirty submodule guidance missing"
[[ "$(cat "$dirty_repo/git.log")" == 'git -C obsidian status --porcelain' ]] \
  || fail "dirty submodule advanced synchronization"

echo 'sync-obsidian-test: all checks passed'
