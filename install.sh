#!/bin/sh
# Installs into user locations only: ~/.local/bin and ~/.local/share.
# Uses the prebuilt binary beside this script if there is one, otherwise builds
# from source. Set CARGO_TARGET_DIR to build somewhere other than ./target (the
# release build needs ~2 GB of scratch space).
set -eu
cd "$(dirname "$0")"
if [ -x ./plaza ] && [ -f ./plaza ]; then
    bin=./plaza
else
    cargo build --release
    bin="${CARGO_TARGET_DIR:-target}/release/plaza"
fi
share="$HOME/.local/share"
install -Dm755 "$bin" "$HOME/.local/bin/plaza"
install -Dm644 assets/plaza.svg "$share/icons/hicolor/scalable/apps/plaza.svg"
install -Dm644 assets/plaza-256.png "$share/icons/hicolor/256x256/apps/plaza.png"
install -Dm644 assets/plaza-64.png "$share/icons/hicolor/64x64/apps/plaza.png"
mkdir -p "$share/applications"
sed "s|@BIN@|$HOME/.local/bin/plaza|" assets/plaza.desktop > "$share/applications/plaza.desktop"
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true
command -v update-desktop-database >/dev/null && update-desktop-database "$share/applications" >/dev/null 2>&1 || true
echo "Installed. Launch Plaza from the app menu, or run: plaza"
