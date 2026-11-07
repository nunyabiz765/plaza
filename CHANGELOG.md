# Changelog

## 0.1.3 (2026-11-07)

- **.deb for Ubuntu, Zorin OS, Linux Mint and Pop!_OS.** `plaza_0.1.3_amd64.deb` installs by double-click from the Software app (or `sudo apt install ./plaza_0.1.3_amd64.deb`) on Ubuntu 22.04 and 24.04 and the distros built on them, with a menu entry, icons and AppStream data.
- **Starts on Ubuntu 22.04.** Linux sound now goes over PulseAudio's protocol (PulseAudio itself or PipeWire's pipewire-pulse), falling back to ALSA. Plaza no longer links libpipewire, whose 22.04 version is too old for cpal's native PipeWire host.
- **Sound Pack 1:** six mid-80s synth-pop presets: Glasshouse Poly, Night Bus Seq, Towpath Bass, Redbrick Strings, Overcast Brass and Platform Bell. `plaza render` gains the phrases `pads8 seq16 hibass topline stabs8 strings8` and a `NAME*N` repeat suffix.

## 0.1.2 (2026-11-06)

- macOS build signed with a Developer ID and notarized by Apple.

## 0.1.1 (2026-11-06)

- Windows and macOS builds.

## 0.1.0 (2026-11-06)

- First release, Linux.
