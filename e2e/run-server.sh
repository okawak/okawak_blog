#!/bin/sh
set -eu

cargo leptos build -p server
exec ./target/debug/server
