#!/bin/bash
# Retakes docs/screenshots on a private Xvfb display. The app runs --muted, so
# the audio engine runs (the scope moves) but every sample sent out is silence.
cd "$(dirname "$0")/.."
EXE=${PLAZA_EXE:-${CARGO_TARGET_DIR:-target}/release/plaza}
OUT=docs/screenshots; mkdir -p "$OUT"
N=78
start() {
  Xvfb ":$N" -screen 0 "${1}x24" -nolisten tcp >/dev/null 2>&1 & XPID=$!
  sleep 0.7
  env -u WAYLAND_DISPLAY DISPLAY=":$N" XDG_SESSION_TYPE=x11 SLINT_BACKEND=winit-software PLAZA_HOLD="$2" \
    "$EXE" --muted --preset "$3" >"/tmp/plaza-shot.log" 2>&1 & APID=$!
  sleep 4
}
stop() { kill $APID $XPID 2>/dev/null; wait $APID $XPID 2>/dev/null; }
shot() { sleep "${2:-1}"; DISPLAY=:$N import -window root "$OUT/$1.png"; }
x() { tools/xclick.py :$N "$@"; }

start 1280x780 "60,64,67,71,74" "Chorus Pad"; shot main 0; x key Tab; shot slowed 1.2; stop
start 1280x780 "60,67,71,76" "Mall Atrium"; shot mall-atrium 0
x click 640 32; shot preset-menu 0.6; stop
start 1280x780 "" "FM Tine Piano"; x down z; x down c; x down b; x down q; shot fm-piano 0.4; x up z; x up c; x up b; x up q; shot fm-released 3; stop
start 1280x780 "" "Mall Atrium"; x click 1245 32; shot about 0.8; stop
