#!/bin/bash
# Builds a .deb from a built Linux binary: package-deb.sh BINARY OUTDIR
# Needs dpkg-dev (dpkg-deb, dpkg-shlibdeps). The binary comes from the Debian 12 build and
# must ask for glibc 2.35 at most, so one .deb installs on Ubuntu 22.04 and 24.04 and the
# distros built on them (Zorin OS 17/18, Linux Mint 21/22, Pop!_OS 22.04/24.04).
set -euo pipefail
cd "$(dirname "$0")/.."
BIN=$1; OUT=$(mkdir -p "$2" && cd "$2" && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DEB="plaza_${VERSION}_amd64.deb"

# Ubuntu 22.04 has glibc 2.35. Fail here rather than ship a .deb it can't run.
GLIBC_FLOOR=2.35
need=$(objdump -T "$BIN" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
if [ "$(printf '%s\n%s\n' "$need" "$GLIBC_FLOOR" | sort -V | tail -1)" != "$GLIBC_FLOOR" ]; then
    echo "plaza needs glibc $need, newer than $GLIBC_FLOOR (Ubuntu 22.04)" >&2; exit 1
fi
# Ubuntu 22.04's libpipewire (0.3.48) lacks symbols cpal's PipeWire host links; see Cargo.toml.
if objdump -p "$BIN" | grep -q 'NEEDED.*libpipewire'; then
    echo "plaza links libpipewire, which won't load on Ubuntu 22.04" >&2; exit 1
fi

STAGE=$(mktemp -d); ROOT="$STAGE/root"
install -Dm755 "$BIN" "$ROOT/usr/bin/plaza"
mkdir -p "$ROOT/usr/share/applications"
sed 's|@BIN@|plaza|' assets/plaza.desktop > "$ROOT/usr/share/applications/plaza.desktop"
chmod 644 "$ROOT/usr/share/applications/plaza.desktop"
for s in 48 64 128 256 512; do
    install -Dm644 "assets/icons/plaza-$s.png" "$ROOT/usr/share/icons/hicolor/${s}x${s}/apps/plaza.png"
done
install -Dm644 assets/plaza.svg "$ROOT/usr/share/icons/hicolor/scalable/apps/plaza.svg"
install -Dm644 assets/plaza.metainfo.xml "$ROOT/usr/share/metainfo/dev.centerpointlabs.plaza.metainfo.xml"

DOC="$ROOT/usr/share/doc/plaza"
install -Dm644 README.md "$DOC/README.md"
install -m644 NOTICE LICENSE-MIT LICENSE-APACHE "$DOC/"
install -Dm644 -t "$DOC/LICENSES" LICENSES/*
python3 -I tools/third-party-licenses.py "$DOC/THIRD-PARTY-LICENSES.txt" x86_64-unknown-linux-gnu
chmod 644 "$DOC/THIRD-PARTY-LICENSES.txt"
cat > "$DOC/copyright" <<'EOF'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: plaza
Source: https://github.com/nunyabiz765/plaza

Files: *
Copyright: 2026 Center Point Labs by nicksosa.tech LLC
License: MIT or Apache-2.0
 Full texts in /usr/share/doc/plaza/LICENSE-MIT and LICENSE-APACHE.
 Slint, the bundled fonts and the Rust crates: /usr/share/doc/plaza/NOTICE,
 LICENSES/ and THIRD-PARTY-LICENSES.txt.
EOF
chmod 644 "$DOC/copyright"

# Libraries the binary links, then the ones winit and Slint open at run time (X11 or
# Wayland, keyboard maps, OpenGL), which dpkg-shlibdeps can't see. ALSA was renamed
# libasound2t64 in Ubuntu 24.04, so that one is an alternative; the rest are the same
# package on Ubuntu 22.04 and 24.04.
mkdir -p "$STAGE/debian"
printf 'Source: plaza\n\nPackage: plaza\nArchitecture: amd64\n' > "$STAGE/debian/control"
linked=$(cd "$STAGE" && dpkg-shlibdeps -O -e root/usr/bin/plaza 2>/dev/null | sed -n 's/^shlibs:Depends=//p')
test -n "$linked" || { echo "dpkg-shlibdeps found nothing" >&2; exit 1; }
linked=$(echo "$linked" | sed -E 's/libasound2( \([^)]*\))?/libasound2\1 | libasound2t64/')
runtime="libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libxrandr2, libxrender1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libwayland-cursor0, libwayland-egl1, libegl1, libgl1"

mkdir -p "$ROOT/DEBIAN"
cat > "$ROOT/DEBIAN/control" <<EOF
Package: plaza
Version: $VERSION
Architecture: amd64
Maintainer: Center Point Labs by nicksosa.tech LLC <nick@centerpointlabs.dev>
Installed-Size: $(du -sk --exclude=DEBIAN "$ROOT" | cut -f1)
Depends: $linked, $runtime
Recommends: pipewire-pulse | pulseaudio, xdg-utils
Section: sound
Priority: optional
Homepage: https://centerpointlabs.dev/services/plaza/
Description: 80s polysynth for vaporwave
 Plaza is an eight-voice synthesizer with an ensemble chorus, a big
 modulated reverb, a cassette stage and a slow + pitch down switch. Play
 it from the computer keyboard or a USB MIDI keyboard, and record what
 you play to ~/Music/Plaza as WAV files. Everything is synthesized; it
 makes no network connections.
EOF
# Tells the desktop to pick up the new menu entry and icon right away.
cat > "$ROOT/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
    command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
    command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
EOF
cp "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/postrm"
sed -i 's/= configure/= remove/' "$ROOT/DEBIAN/postrm"
chmod 755 "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/postrm"

dpkg-deb --root-owner-group -Zxz --build "$ROOT" "$OUT/$DEB"
dpkg-deb --info "$OUT/$DEB"
(cd "$OUT" && sha256sum "$DEB" > "$DEB.sha256" && cat "$DEB.sha256")
rm -rf "$STAGE"
