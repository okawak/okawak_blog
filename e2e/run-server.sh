#!/bin/sh
set -eu

if [ "${OKAWAK_BLOG_E2E_REUSE_BUILD:-false}" != "true" ] \
  || [ ! -x ./target/debug/topcoat-server ] \
  || [ ! -s ./target/debug/assets/manifest.toml ]; then
  topcoat asset bundle --package server --bin topcoat-server
fi
exec ./target/debug/topcoat-server
