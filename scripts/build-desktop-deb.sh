#!/bin/sh
set -eu

VERSION="0.7.0"
PKG="ruos-desktop"
ARCH="amd64"
DEB_DIR="$(cd "$(dirname "$0")/.." && pwd)/out/deb"
STAGE="$(mktemp -d)"

echo "==> Building ${PKG}_${VERSION}_${ARCH}.deb"

# Check for the Tauri .deb or binary
TAURI_DEB="/home/ruvultra/projects/ruvultra-cognitum/app/src-tauri/target/release/bundle/deb/ruVultra_0.1.0_amd64.deb"
TAURI_BIN="/home/ruvultra/projects/ruvultra-cognitum/app/src-tauri/target/release/ruvultra-app"

mkdir -p "${STAGE}/DEBIAN"
mkdir -p "${STAGE}/usr/local/bin"
mkdir -p "${STAGE}/usr/share/applications"
mkdir -p "${STAGE}/usr/share/icons/hicolor/128x128/apps"

# Copy the Tauri binary
if [ -f "$TAURI_BIN" ]; then
  cp "$TAURI_BIN" "${STAGE}/usr/local/bin/ruos-desktop"
  chmod 755 "${STAGE}/usr/local/bin/ruos-desktop"
else
  echo "ERROR: Tauri binary not found at $TAURI_BIN"
  exit 1
fi

# Desktop entry
cat > "${STAGE}/usr/share/applications/ruos-desktop.desktop" << 'DESKTOP'
[Desktop Entry]
Name=ruOS
Comment=AI workstation dashboard — GPU, brain, profiles, search
Exec=ruos-desktop
Icon=ruos
Type=Application
Categories=System;Monitor;
Terminal=false
DESKTOP

# Icon (use the existing gold icon if available)
if [ -f "/home/ruvultra/projects/ruvultra-cognitum/app/src-tauri/icons/128x128.png" ]; then
  cp "/home/ruvultra/projects/ruvultra-cognitum/app/src-tauri/icons/128x128.png" "${STAGE}/usr/share/icons/hicolor/128x128/apps/ruos.png"
fi

# Control file
cat > "${STAGE}/DEBIAN/control" << EOF
Package: ${PKG}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: ruv <ruv@ruv.net>
Depends: ruos-core, libwebkit2gtk-4.1-0, libgtk-3-0
Description: ruOS desktop app — Tauri-based AI workstation dashboard
 Gold neural theme dashboard for monitoring GPU, brain, profiles, and
 semantic search. Built with Tauri v2 + Svelte 5. Connects to local
 ruvultra services via loopback HTTP.
Homepage: https://github.com/cognitum-one/ruVultra
EOF

mkdir -p "$DEB_DIR"
dpkg-deb --build "$STAGE" "${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb"
SIZE=$(stat -c%s "${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb")
echo "==> Built: ${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb (${SIZE} bytes)"

rm -rf "$STAGE"
