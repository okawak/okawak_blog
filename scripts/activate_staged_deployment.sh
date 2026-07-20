#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

service_name="${SERVICE_NAME:-okawak_blog}"
service_file="${SERVICE_FILE:-service/okawak_blog.service}"
target_bin="${TARGET_BIN:-./target/release/server}"
target_hash_file="${TARGET_HASH_FILE:-./target/release/hash.txt}"
bin_dir="${BIN_DIR:-./bin}"
staged_site="${DEPLOY_STAGED_SITE:-./target/site-staged}"
live_site="./target/site"
previous_site="./target/site-previous"
older_previous_site="./target/site-previous-rollback"
failed_site="./target/site-failed"
installed_bin="$bin_dir/$service_name"
rollback_bin="$bin_dir/$service_name.rollback"
installed_hash_file="$bin_dir/hash.txt"
rollback_hash_file="$bin_dir/hash.txt.rollback"

service_was_active=false
live_site_moved=false
previous_site_moved=false
site_swapped=false
binary_change_started=false
had_installed_bin=false
had_installed_hash_file=false

fail() {
  echo "staged-deploy: $*" >&2
  exit 1
}

path_exists() {
  [[ -e "$1" || -L "$1" ]]
}

rollback() {
  local status="$1"
  local line="$2"

  trap - ERR
  set +e
  echo "staged-deploy: activation failed at line $line; restoring the previous release" >&2

  sudo systemctl stop "$service_name.service"

  if [[ "$binary_change_started" == true ]]; then
    if [[ "$had_installed_bin" == true ]]; then
      sudo mv -f "$rollback_bin" "$installed_bin"
    else
      sudo rm -f "$installed_bin"
    fi
    if [[ "$had_installed_hash_file" == true ]]; then
      sudo mv -f "$rollback_hash_file" "$installed_hash_file"
    else
      sudo rm -f "$installed_hash_file"
    fi
  else
    sudo rm -f "$rollback_bin" "$rollback_hash_file"
  fi

  if [[ "$site_swapped" == true ]]; then
    if ! path_exists "$failed_site"; then
      mv "$live_site" "$failed_site"
    fi
  fi
  if [[ "$live_site_moved" == true ]]; then
    mv "$previous_site" "$live_site"
  fi
  if [[ "$previous_site_moved" == true ]]; then
    mv "$older_previous_site" "$previous_site"
  fi

  sudo systemctl daemon-reload
  if [[ "$service_was_active" == true ]]; then
    sudo systemctl start "$service_name.service"
  fi

  echo "staged-deploy: failed release is preserved at $failed_site when available" >&2
  exit "$status"
}

path_exists "$staged_site" || fail "staged site is missing: $staged_site"
[[ -d "$staged_site/pkg" ]] || fail "staged pkg directory is missing"
[[ -f "$staged_site/favicon.ico" ]] || fail "staged favicon is missing"
[[ -x "$target_bin" ]] || fail "server binary is missing: $target_bin"
[[ -f "$target_hash_file" ]] || fail "asset hash manifest is missing: $target_hash_file"
find "$staged_site/pkg" -maxdepth 1 -type f -name '*.js' -print -quit | grep -q . \
  || fail "staged JavaScript bundle is missing"
find "$staged_site/pkg" -maxdepth 1 -type f -name '*.wasm' -print -quit | grep -q . \
  || fail "staged WebAssembly bundle is missing"

path_exists "$older_previous_site" \
  && fail "older previous site already exists: $older_previous_site"
path_exists "$failed_site" && fail "failed site already exists: $failed_site"
path_exists "$rollback_bin" && fail "rollback binary already exists: $rollback_bin"
path_exists "$rollback_hash_file" && fail "rollback hash manifest already exists: $rollback_hash_file"

if sudo systemctl is-active --quiet "$service_name.service"; then
  service_was_active=true
fi

trap 'rollback $? $LINENO' ERR

sudo install -o root -g root -m 0644 \
  "$service_file" "/etc/systemd/system/$service_name.service"
sudo systemctl daemon-reload
sudo systemctl stop "$service_name.service"

if path_exists "$previous_site"; then
  mv "$previous_site" "$older_previous_site"
  previous_site_moved=true
fi
if path_exists "$live_site"; then
  mv "$live_site" "$previous_site"
  live_site_moved=true
fi
mv "$staged_site" "$live_site"
site_swapped=true

sudo mkdir -p "$bin_dir"
if path_exists "$installed_bin"; then
  sudo cp -p "$installed_bin" "$rollback_bin"
  had_installed_bin=true
fi
if path_exists "$installed_hash_file"; then
  sudo cp -p "$installed_hash_file" "$rollback_hash_file"
  had_installed_hash_file=true
fi
binary_change_started=true
sudo install -o root -g root -m 0755 "$target_bin" "$installed_bin"
sudo install -o root -g root -m 0644 "$target_hash_file" "$installed_hash_file"

sudo systemctl daemon-reload
sudo systemctl start "$service_name.service"

ready=false
for _ in {1..15}; do
  if curl --fail --silent --show-error --output /dev/null \
    http://127.0.0.1:8008/api/health \
    && curl --fail --silent --show-error --output /dev/null \
      http://127.0.0.1:8008/api/ready; then
    ready=true
    break
  fi
  sleep 1
done
[[ "$ready" == true ]] || fail "health/readiness checks did not pass"

trap - ERR
if [[ "$previous_site_moved" == true ]]; then
  rm -rf -- "$older_previous_site"
fi
if [[ "$had_installed_bin" == true ]]; then
  sudo rm -f "$rollback_bin"
fi
if [[ "$had_installed_hash_file" == true ]]; then
  sudo rm -f "$rollback_hash_file"
fi

echo "staged-deploy: release activated and health/readiness checks passed"
