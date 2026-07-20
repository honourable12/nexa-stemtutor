#!/bin/bash
set -e

# Navigate to Tauri directory and compile release binary
cd "$(dirname "$0")/src-tauri"
cargo build --release