#!/usr/bin/env bash
set -euo pipefail

VERSION=$(node -p "require('./package.json').version")

# Sync to Cargo.toml
sed -i "0,/^version = /s/^version = .*/version = \"${VERSION}\"/" src-tauri/Cargo.toml

# Sync to tauri.conf.json
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/" src-tauri/tauri.conf.json

echo "Synced version ${VERSION} to Cargo.toml and tauri.conf.json"
