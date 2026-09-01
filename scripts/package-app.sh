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

iconset_dir="$stage_dir/CodexSkinLite.iconset"
mkdir -p "$iconset_dir"
icon_sizes=(
  "16 16x16"
  "32 16x16@2x"
  "32 32x32"
  "64 32x32@2x"
  "128 128x128"
  "256 128x128@2x"
  "256 256x256"
  "512 256x256@2x"
  "512 512x512"
  "1024 512x512@2x"
)
for spec in "${icon_sizes[@]}"; do
  read -r size name <<< "$spec"
  sips -z "$size" "$size" resources/CodexSkinLite-icon.png \
    --out "$iconset_dir/icon_${name}.png" >/dev/null
done
iconutil -c icns "$iconset_dir" \
  -o "$app_dir/Contents/Resources/CodexSkinLite.icns"

cp target/aarch64-apple-darwin/release/codex-skin-lite \
  "$app_dir/Contents/MacOS/CodexSkinLite"
cp resources/Info.plist "$app_dir/Contents/Info.plist"
chmod 755 "$app_dir/Contents/MacOS/CodexSkinLite"
plutil -lint "$app_dir/Contents/Info.plist"
file "$app_dir/Contents/MacOS/CodexSkinLite" | grep -q 'arm64'

signing_identity="${CODESIGN_IDENTITY:-}"
if [[ -z "$signing_identity" ]]; then
  signing_identity="$(security find-identity -v -p codesigning \
    | awk -F '"' '/Developer ID Application:/ { print $2; exit }')"
fi
if [[ -z "$signing_identity" ]]; then
  echo "No Developer ID Application signing identity found" >&2
  exit 1
fi
codesign --force --options runtime --timestamp \
  --sign "$signing_identity" "$app_dir"
codesign --verify --deep --strict --verbose=2 "$app_dir"

version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
archive="dist/CodexSkinLite-${version}-macos-arm64.zip"
rm -f "$archive" "$archive.sha256"
ditto -c -k --sequesterRsrc --keepParent \
  "$app_dir" "$archive"
shasum -a 256 "$archive" > "$archive.sha256"

echo "Created $archive"
