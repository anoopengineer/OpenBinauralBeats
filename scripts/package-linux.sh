#!/usr/bin/env bash
# Packages target/release/open-binaural-beats as a .tar.gz, .deb and AppImage in dist/.
# Needs cargo-deb and appimagetool on PATH (CI installs both).
set -euo pipefail
cd "$(dirname "$0")/.."

BIN=open-binaural-beats
DESKTOP=packaging/linux/$BIN.desktop
DIST=dist
rm -rf "$DIST" && mkdir -p "$DIST"

# Tarball
STAGE=$(mktemp -d)/OpenBinauralBeats
mkdir -p "$STAGE"
cp "target/release/$BIN" "$DESKTOP" LICENSE "$STAGE/"
cp assets/icon-256.png "$STAGE/$BIN.png"
tar -C "$(dirname "$STAGE")" -czf "$DIST/OpenBinauralBeats-linux-x86_64.tar.gz" OpenBinauralBeats

# Debian package (reuses the existing release build)
cargo deb --no-build --output "$DIST/OpenBinauralBeats-linux-amd64.deb"

# AppImage
APPDIR=$(mktemp -d)/OpenBinauralBeats.AppDir
mkdir -p "$APPDIR/usr/bin"
cp "target/release/$BIN" "$APPDIR/usr/bin/"
cp "$DESKTOP" "$APPDIR/"
cp assets/icon-256.png "$APPDIR/$BIN.png"
ln -s "usr/bin/$BIN" "$APPDIR/AppRun"
ARCH=x86_64 appimagetool --appimage-extract-and-run "$APPDIR" "$DIST/OpenBinauralBeats-linux-x86_64.AppImage"

ls -lh "$DIST"
