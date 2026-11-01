#!/bin/sh
set -eu

if [ "${OKAWAK_BLOG_E2E_REUSE_BUILD:-false}" != "true" ] \
  || [ ! -x ./target/debug/blog-server ] \
  || [ ! -s ./target/debug/assets/manifest.toml ]; then
  topcoat asset bundle --package blog-server --bin blog-server
fi
exec ./target/debug/blog-server
