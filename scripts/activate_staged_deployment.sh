#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$repo_root"

service_name="${SERVICE_NAME:-okawak_blog}"
service_file="${SERVICE_FILE:-service/okawak_blog.service}"
systemd_unit_dir="${SYSTEMD_UNIT_DIR:-/etc/systemd/system}"
target_bin="${TARGET_BIN:-./target/release/server}"
bin_dir="${BIN_DIR:-$repo_root/bin}"
[[ "$bin_dir" == /* ]] || bin_dir="$repo_root/$bin_dir"
staged_assets="${DEPLOY_STAGED_ASSETS:-./target/assets-staged}"
live_assets="$bin_dir/assets"
rollback_assets="$bin_dir/assets.rollback"
failed_assets="$bin_dir/assets.failed"
installed_bin="$bin_dir/$service_name"
rollback_bin="$bin_dir/$service_name.rollback"
installed_service="$systemd_unit_dir/$service_name.service"
rollback_service="$installed_service.rollback"
probe_attempts="${DEPLOY_PROBE_ATTEMPTS:-15}"

service_was_active=false
live_assets_moved=false
assets_swapped=false
binary_change_started=false
had_installed_bin=false
unit_change_started=false
had_installed_service=false

fail() {
  echo "staged-deploy: $*" >&2
  return 1
}

# Keep deployment paths literal in both the shell and systemd unit syntax.
for deployment_path in "$repo_root" "$installed_bin"; do
  [[ "$deployment_path" =~ ^/[A-Za-z0-9._/-]+$ && "$deployment_path" != / ]] \
    || fail "deployment paths must be absolute and contain only letters, digits, '/', '.', '_', and '-'"
done

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
  else
    sudo rm -f "$rollback_bin"
  fi

  if [[ "$assets_swapped" == true ]]; then
    if ! path_exists "$failed_assets"; then
      sudo mv "$live_assets" "$failed_assets"
    fi
  fi
  if [[ "$live_assets_moved" == true ]]; then
    sudo mv "$rollback_assets" "$live_assets"
  fi

  if [[ "$unit_change_started" == true ]]; then
    if [[ "$had_installed_service" == true ]]; then
      if ! sudo mv -f "$rollback_service" "$installed_service"; then
        echo "staged-deploy: unit recovery failed; restore $rollback_service before restarting the service" >&2
        exit "$status"
      fi
    elif ! sudo rm -f "$installed_service"; then
      echo "staged-deploy: could not remove the failed deployment unit: $installed_service" >&2
      exit "$status"
    fi
  fi

  sudo systemctl daemon-reload
  if [[ "$service_was_active" == true ]]; then
    sudo systemctl start "$service_name.service"
  fi

  echo "staged-deploy: failed asset bundle is preserved at $failed_assets when available" >&2
  exit "$status"
}

path_exists "$staged_assets" || fail "staged asset bundle is missing: $staged_assets"
[[ -f "$staged_assets/manifest.toml" ]] || fail "asset manifest is missing"
[[ -x "$target_bin" ]] || fail "server binary is missing: $target_bin"
grep -q '^content_type = "text/css"$' "$staged_assets/manifest.toml" \
  || fail "staged CSS asset is missing"
grep -q '^content_type = "text/javascript"$' "$staged_assets/manifest.toml" \
  || fail "staged JavaScript asset is missing"
grep -q '^content_type = "image/x-icon"$' "$staged_assets/manifest.toml" \
  || fail "staged favicon asset is missing"
if find "$staged_assets" -maxdepth 1 -type f -name '*.wasm' -print -quit | grep -q . \
  || grep -q '^content_type = "application/wasm"$' "$staged_assets/manifest.toml"; then
  fail "staged asset bundle must not contain WebAssembly"
fi

asset_count=0
while IFS= read -r asset_file; do
  [[ -n "$asset_file" ]] || continue
  [[ "$asset_file" != */* && "$asset_file" != "." && "$asset_file" != ".." ]] \
    || fail "asset manifest contains an unsafe filename: $asset_file"
  [[ -f "$staged_assets/$asset_file" ]] \
    || fail "manifest asset is missing: $asset_file"
  ((asset_count += 1))
done < <(sed -nE 's/^file = "([^"]+)"$/\1/p' "$staged_assets/manifest.toml")
((asset_count > 0)) || fail "asset manifest contains no files"

path_exists "$rollback_assets" && fail "rollback asset bundle already exists: $rollback_assets"
path_exists "$failed_assets" && fail "failed asset bundle already exists: $failed_assets"
path_exists "$rollback_bin" && fail "rollback binary already exists: $rollback_bin"
path_exists "$rollback_service" && fail "rollback systemd unit already exists: $rollback_service"

rendered_service="$(mktemp)"
trap 'rm -f -- "$rendered_service"' EXIT
sed \
  -e "s|^WorkingDirectory=.*|WorkingDirectory=$repo_root|" \
  -e "s|^ExecStart=.*|ExecStart=$installed_bin|" \
  "$service_file" >"$rendered_service"

if sudo systemctl is-active --quiet "$service_name.service"; then
  service_was_active=true
fi

# Preserve the old unit before changing it, including when it points to another checkout.
if path_exists "$installed_service"; then
  sudo cp -Pp "$installed_service" "$rollback_service"
  had_installed_service=true
fi

trap 'rollback $? $LINENO' ERR

unit_change_started=true
sudo install -o root -g root -m 0644 \
  "$rendered_service" "$installed_service"
sudo systemctl daemon-reload
sudo systemctl stop "$service_name.service"

sudo mkdir -p "$bin_dir"
if path_exists "$live_assets"; then
  sudo mv "$live_assets" "$rollback_assets"
  live_assets_moved=true
fi
sudo mv "$staged_assets" "$live_assets"
assets_swapped=true
sudo chown -R root:root "$live_assets"

if path_exists "$installed_bin"; then
  sudo cp -p "$installed_bin" "$rollback_bin"
  had_installed_bin=true
fi
binary_change_started=true
sudo install -o root -g root -m 0755 "$target_bin" "$installed_bin"

sudo systemctl daemon-reload
sudo systemctl start "$service_name.service"

sleep 1
ready=false
for ((attempt = 1; attempt <= probe_attempts; attempt += 1)); do
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
if [[ "$live_assets_moved" == true ]]; then
  sudo rm -rf -- "$rollback_assets"
fi
if [[ "$had_installed_bin" == true ]]; then
  sudo rm -f "$rollback_bin"
fi
if [[ "$had_installed_service" == true ]]; then
  sudo rm -f "$rollback_service"
fi

echo "staged-deploy: release activated and health/readiness checks passed"
