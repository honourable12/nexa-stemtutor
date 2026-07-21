#!/bin/bash
set -e

cd "$(dirname "$0")/src-tauri"
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release