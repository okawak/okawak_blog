#!/bin/sh
set -eu

if [ "${OKAWAK_BLOG_E2E_REUSE_BUILD:-false}" != "true" ] \
  || [ ! -x ./target/debug/site-server ] \
  || [ ! -s ./target/debug/assets/manifest.toml ]; then
  topcoat asset bundle --package server --bin site-server
fi
exec ./target/debug/site-server
