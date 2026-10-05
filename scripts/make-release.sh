#!/bin/sh
# Builds a release on this machine: an installer for each processor, their update signatures
# and latest.json, which the panel reads to learn about a new version. Everything lands in dist/.
# The update signing key is read from ~/.tapka/updater.key (it never leaves this machine).
# Usage: sh scripts/make-release.sh
set -e
cd "$(dirname "$0")/.."
version=$(grep -m1 '"version"' tauri.conf.json | sed 's/[^0-9.]//g')
export TAURI_SIGNING_PRIVATE_KEY="$(cat "$HOME/.tapka/updater.key")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
rm -rf dist && mkdir dist
for pair in aarch64-pc-windows-msvc:arm64 x86_64-pc-windows-msvc:x64; do
  target=${pair%%:*}; arch=${pair##*:}
  npx --yes @tauri-apps/cli@2 build --target "$target"
  cp "target/$target/release/bundle/nsis/Tapka_${version}_${arch}-setup.exe" "target/$target/release/bundle/nsis/Tapka_${version}_${arch}-setup.exe.sig" dist/
done
base="https://github.com/Sargares22/tapka/releases/download/v$version"
cat > dist/latest.json <<EOF
{
  "version": "$version",
  "notes": "See CHANGELOG.md",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "platforms": {
    "windows-aarch64": { "signature": "$(cat dist/Tapka_${version}_arm64-setup.exe.sig)", "url": "$base/Tapka_${version}_arm64-setup.exe" },
    "windows-x86_64": { "signature": "$(cat dist/Tapka_${version}_x64-setup.exe.sig)", "url": "$base/Tapka_${version}_x64-setup.exe" }
  }
}
EOF
ls -la dist
