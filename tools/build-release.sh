#!/bin/bash
# Builds the x86_64 Linux release tarball in a Debian 12 container, so the binary
# asks for an older glibc than a rolling distro would give it. Usage: build-release.sh OUTDIR
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=$(mkdir -p "$1" && cd "$1" && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME="plaza-$VERSION-x86_64-linux"

podman run --rm -v "$PWD:/src:ro" -v "$OUT:/out" docker.io/library/debian:12 bash -euc '
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends build-essential pkg-config curl ca-certificates \
    clang libclang-dev libasound2-dev libpipewire-0.3-dev libspa-0.2-dev libpulse-dev \
    libfontconfig1-dev libfreetype6-dev libxkbcommon-dev >/dev/null
  curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal >/dev/null
  . "$HOME/.cargo/env"
  rustc --version
  cd /src
  CARGO_TARGET_DIR=/tmp/target cargo build --release --locked
  cp /tmp/target/release/plaza /out/plaza.bin
'

python3 -I tools/third-party-licenses.py "$OUT/THIRD-PARTY-LICENSES.txt"
STAGE=$(mktemp -d); DIR="$STAGE/$NAME"; mkdir -p "$DIR/assets"
mv "$OUT/plaza.bin" "$DIR/plaza"; mv "$OUT/THIRD-PARTY-LICENSES.txt" "$DIR/"
cp install.sh README.md NOTICE LICENSE-MIT LICENSE-APACHE "$DIR/"
cp -r LICENSES "$DIR/"
cp assets/plaza.svg assets/plaza-256.png assets/plaza-64.png assets/plaza.desktop "$DIR/assets/"
chmod 755 "$DIR/plaza" "$DIR/install.sh"
tar -C "$STAGE" --owner=0 --group=0 --sort=name -czf "$OUT/$NAME.tar.gz" "$NAME"
(cd "$OUT" && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256" && cat "$NAME.tar.gz.sha256")
rm -rf "$STAGE"
