#!/bin/bash
# Builds Plaza.app from an arm64 and an x86_64 binary, ad-hoc signs it, and wraps
# it in a .dmg. Runs on macOS: package-macos.sh ARM64_BIN X86_64_BIN OUTDIR
set -euo pipefail
cd "$(dirname "$0")/.."
ARM=$1; X86=$2; OUT=$(mkdir -p "$3" && cd "$3" && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME="plaza-$VERSION-macos-universal"
STAGE=$(mktemp -d); APP="$STAGE/Plaza.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$STAGE/Licenses"
lipo -create -output "$APP/Contents/MacOS/plaza" "$ARM" "$X86"
lipo -info "$APP/Contents/MacOS/plaza"
cp assets/plaza.icns "$APP/Contents/Resources/plaza.icns"
sed "s/@VERSION@/$VERSION/g" tools/dist/Info.plist > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
# Ad-hoc signature: no Apple identity, so Gatekeeper still asks once (see READ ME FIRST).
# Apple Silicon refuses to run a binary with no signature at all.
codesign --force --deep --sign - "$APP"
codesign --verify --verbose=2 "$APP"
python3 -I tools/third-party-licenses.py "$STAGE/Licenses/THIRD-PARTY-LICENSES.txt" aarch64-apple-darwin x86_64-apple-darwin
cp README.md NOTICE LICENSE-MIT LICENSE-APACHE "$STAGE/Licenses/"
cp -r LICENSES/. "$STAGE/Licenses/"
cp tools/dist/READ-ME-FIRST-macos.txt "$STAGE/READ ME FIRST.txt"
ln -s /Applications "$STAGE/Applications"
rm -f "$OUT/$NAME.dmg"
hdiutil create -volname "Plaza $VERSION" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$OUT/$NAME.dmg" >/dev/null
(cd "$OUT" && shasum -a 256 "$NAME.dmg" > "$NAME.dmg.sha256" && cat "$NAME.dmg.sha256")
rm -rf "$STAGE"
