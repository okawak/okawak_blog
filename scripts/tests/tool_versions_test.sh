#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT

mkdir -p "$test_root/bin" "$test_root/crates/server" "$test_root/.github/workflows"
cat >"$test_root/mise.toml" <<'EOF'
[tools]
bun = "1.0.0"
"cargo:topcoat-cli" = "0.1.0"
[env]
TOPCOAT_TAILWIND_VERSION = "4.0.0"
EOF
cat >"$test_root/Cargo.toml" <<'EOF'
[workspace.dependencies]
topcoat = { version = "=0.1.0" }
EOF
printf '.version("4.0.0")\n' >"$test_root/crates/server/build.rs"
cat >"$test_root/bin/bun" <<'EOF'
#!/usr/bin/env bash
printf '1.0.0\n'
EOF
cat >"$test_root/bin/topcoat" <<'EOF'
#!/usr/bin/env bash
printf 'topcoat 0.1.0\n'
EOF
chmod +x "$test_root/bin/bun" "$test_root/bin/topcoat"
export PATH="$test_root/bin:$PATH"

write_workflow() {
  local workflow="$1"
  shift
  printf 'jobs:\n  verify:\n    steps:\n' >"$test_root/$workflow"
  for ref in "$@"; do
    printf '      - name: Set up build tools\n        uses: jdx/mise-action@%s\n' "$ref" >>"$test_root/$workflow"
  done
}

run_case() {
  local expected_status="$1" description="$2" status=0
  (cd "$test_root" && bash "$repo_root/scripts/check_tool_versions.sh") >"$test_root/output.log" 2>&1 || status=$?
  if [[ "$status" != "$expected_status" ]]; then
    echo "tool-versions-test: $description: expected $expected_status, got $status" >&2
    cat "$test_root/output.log" >&2
    exit 1
  fi
}

ci_workflow=.github/workflows/ci.yml
upload_workflow=.github/workflows/upload.yml
action_sha=0123456789abcdef0123456789abcdef01234567

write_workflow "$ci_workflow" v4 v4
write_workflow "$upload_workflow" v4
run_case 0 'current major tags'

write_workflow "$ci_workflow" "$action_sha # v4" "$action_sha # v4.2.0"
write_workflow "$upload_workflow" "$action_sha # v4"
run_case 0 'SHA pins with current-major comments'

for workflow in "$ci_workflow" "$upload_workflow"; do
  for invalid_ref in v1 v5 "$action_sha" "$action_sha # v1" "$action_sha # v40" '0123456 # v4'; do
    write_workflow "$ci_workflow" v4 v4
    write_workflow "$upload_workflow" v4
    write_workflow "$workflow" "$invalid_ref"
    run_case 1 "invalid ref in $workflow: $invalid_ref"
    # One valid step must not hide another step using an invalid ref.
    write_workflow "$workflow" v4 "$invalid_ref"
    run_case 1 "mixed refs in $workflow: $invalid_ref"
  done
  write_workflow "$ci_workflow" v4
  write_workflow "$upload_workflow" v4
  write_workflow "$workflow"
  run_case 1 "missing mise action in $workflow"
done

echo 'tool-versions-test: all cases passed'
