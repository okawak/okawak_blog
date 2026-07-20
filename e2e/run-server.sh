#!/bin/sh
set -eu

cargo leptos build -p server
mkdir -p target/site-previous/pkg
printf 'previous release asset\n' > target/site-previous/pkg/e2e-previous-release.txt
exec ./target/debug/server
