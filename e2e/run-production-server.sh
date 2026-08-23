#!/bin/sh
set -eu

if [ "${OKAWAK_BLOG_E2E_REUSE_BUILD:-false}" != "true" ] \
  || [ ! -x ./target/debug/server ]; then
  cargo leptos build -p server
fi
exec ./target/debug/server
