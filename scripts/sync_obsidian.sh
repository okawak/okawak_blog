#!/usr/bin/env bash

set -euo pipefail

legacy_obsidian_path="crates/publish/obsidian"
obsidian_path="obsidian"

if [[ -e "$legacy_obsidian_path" || -L "$legacy_obsidian_path" ]]; then
  echo "sync-obsidian: legacy path $legacy_obsidian_path remains; preserve its changes, then move or remove it before syncing" >&2
  exit 1
fi
if [[ ! -e "$obsidian_path/.git" ]]; then
  git submodule update --init --recursive "$obsidian_path"
fi
if [[ -n "$(git -C "$obsidian_path" status --porcelain)" ]]; then
  echo "sync-obsidian: submodule has uncommitted changes; commit or stash them first" >&2
  exit 1
fi
git submodule update --remote --checkout --recursive "$obsidian_path"
