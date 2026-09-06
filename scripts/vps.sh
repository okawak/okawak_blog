#!/usr/bin/env bash

set -euo pipefail

fail() {
  echo "vps: $*" >&2
  exit 1
}

(($# >= 1 && $# <= 2)) || fail "expected operation and optional SSH target"
operation="$1"
shift
case "$operation" in
  deploy|status|logs|logs-recent|restart) ;;
  *) fail "unsupported operation: $operation" ;;
esac
if [[ "${1:-}" == --help || "${1:-}" == -h ]]; then
  cat <<'EOF'
Usage: mise run <deploy-vps|status-vps|logs-vps|logs-recent-vps|restart-vps> [SSH_TARGET]

Run from the management host. Build and deploy origin/main on the VPS using
the existing production-deploy task in the configured VPS repository directory.

SSH target: argument > OKAWAK_BLOG_VPS_SSH_TARGET (required; no default).
Configure OKAWAK_BLOG_VPS_SSH_TARGET in the management host's mise.local.toml [env].
VPS directory: OKAWAK_BLOG_VPS_REPO_DIR (required for deploy; no default).
Configure it in the management host's mise.local.toml [env].
Use an absolute path containing only letters, digits, '/', '.', '_', and '-'.
SSH config supplies the port, hostname, user and authentication settings.
The VPS login shell must provide mise and its configured build tools.
Keep the terminal connected until deployment finishes; sudo may prompt.
EOF
  exit 0
fi

ssh_target="${1:-${OKAWAK_BLOG_VPS_SSH_TARGET:-}}"
[[ -n "$ssh_target" ]] \
  || fail "set OKAWAK_BLOG_VPS_SSH_TARGET in mise.local.toml [env], or pass an SSH target"
[[ "$ssh_target" =~ ^[A-Za-z0-9._@:-]+$ && "$ssh_target" != -* ]] \
  || fail "SSH target contains unsupported characters: $ssh_target"
# Only the operation and deployment path are sent; local source and keys are not copied.
# Pass the script as an argument, keeping SSH stdin available for sudo prompts.
case "$operation" in
deploy)
  repo_dir="${OKAWAK_BLOG_VPS_REPO_DIR:-}"
  [[ -n "$repo_dir" ]] \
    || fail "set OKAWAK_BLOG_VPS_REPO_DIR in mise.local.toml [env]"
  [[ "$repo_dir" =~ ^/[A-Za-z0-9._/-]+$ && "$repo_dir" != / ]] \
    || fail "OKAWAK_BLOG_VPS_REPO_DIR must be an absolute path containing only letters, digits, '/', '.', '_', and '-'"
  remote_script="set -euo pipefail
cd \"$repo_dir\"
$(cat <<'EOF'
command -v mise >/dev/null || { echo "deploy-vps: mise is missing from the VPS login shell PATH" >&2; exit 1; }
branch="$(git branch --show-current)"
[[ "$branch" == main ]] || { echo "deploy-vps: VPS checkout must be on main" >&2; exit 1; }
changes="$(git status --porcelain)"
[[ -z "$changes" ]] || { echo "deploy-vps: VPS checkout has uncommitted changes" >&2; exit 1; }
git fetch origin main
git merge-base --is-ancestor HEAD FETCH_HEAD || { echo "deploy-vps: VPS has commits outside origin/main; resolve them before deploying" >&2; exit 1; }
exec mise run production-deploy
EOF
)"
  ;;
status) remote_script='exec sudo systemctl status --no-pager okawak_blog' ;;
logs) remote_script='exec sudo journalctl -u okawak_blog --no-pager -f' ;;
logs-recent) remote_script='exec sudo journalctl -u okawak_blog --no-pager --lines=50' ;;
restart) remote_script='exec sudo systemctl restart okawak_blog' ;;
esac
# POSIX shell quoting for the SSH user's outer shell; the script itself uses Bash.
remote_script="${remote_script//\'/\'\\\'\'}"
echo "vps: $operation on $ssh_target"
exec ssh -tt "$ssh_target" "bash -lc '$remote_script'"
