#!/bin/bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd -P)"
cd "$repo_dir"

if [[ "$(uname -m)" != "arm64" ]]; then
  echo "CodexSkinLite packaging requires Apple Silicon" >&2
  exit 1
fi

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
npm ci --prefix renderer
npm test --prefix renderer
node scripts/measure-renderer.mjs
cargo build --release --target aarch64-apple-darwin

stage_dir="$(mktemp -d)"
trap 'rm -rf "$stage_dir"' EXIT
app_dir="$stage_dir/CodexSkinLite.app"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources" dist
cp target/aarch64-apple-darwin/release/codex-skin-lite \
  "$app_dir/Contents/MacOS/CodexSkinLite"
cp resources/Info.plist "$app_dir/Contents/Info.plist"
chmod 755 "$app_dir/Contents/MacOS/CodexSkinLite"
plutil -lint "$app_dir/Contents/Info.plist"
file "$app_dir/Contents/MacOS/CodexSkinLite" | grep -q 'arm64'

version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
archive="dist/CodexSkinLite-${version}-macos-arm64.zip"
rm -f "$archive" "$archive.sha256"
ditto -c -k --sequesterRsrc --keepParent \
  "$app_dir" "$archive"
shasum -a 256 "$archive" > "$archive.sha256"

echo "Created $archive"
