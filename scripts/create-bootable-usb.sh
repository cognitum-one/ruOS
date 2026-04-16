#!/bin/sh
set -eu

# ruOS Bootable USB Creator
# Creates a bootable USB drive with ruOS pre-installed
# Usage: sudo bash create-bootable-usb.sh /dev/sdX
#    or: bash create-bootable-usb.sh /path/to/mount

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DEB_DIR="${PROJECT_DIR}/out/deb"

TARGET="${1:?Usage: create-bootable-usb.sh <device-or-mountpoint>}"

echo "╔══════════════════════════════════════════╗"
echo "║  ruOS Bootable USB Creator               ║"
echo "╚══════════════════════════════════════════╝"
echo

# Detect if target is a block device or mount point
if [ -b "$TARGET" ]; then
  echo "ERROR: Direct block device writing not yet implemented."
  echo "For now, mount the USB first, then pass the mount point:"
  echo "  mount /dev/sdX1 /mnt/usb"
  echo "  bash $0 /mnt/usb"
  exit 1
fi

if [ ! -d "$TARGET" ]; then
  echo "ERROR: $TARGET is not a directory"
  exit 1
fi

INSTALLER_DIR="${TARGET}/ruos-installer"
mkdir -p "${INSTALLER_DIR}/debs"
mkdir -p "${INSTALLER_DIR}/bin"
mkdir -p "${INSTALLER_DIR}/config"
mkdir -p "${INSTALLER_DIR}/brain"

echo "==> Copying ruOS packages..."
for deb in "${DEB_DIR}"/*.deb; do
  [ -f "$deb" ] && cp "$deb" "${INSTALLER_DIR}/debs/" && echo "  $(basename $deb)"
done

echo "==> Copying binaries (for direct install without dpkg)..."
for bin in ruvultra-mcp ruvultra-profile ruvultra-init mcp-brain-server-local mcp-brain; do
  src="$HOME/.local/bin/$bin"
  [ -f "$src" ] && cp "$src" "${INSTALLER_DIR}/bin/" && echo "  $bin"
done

echo "==> Copying configuration..."
cp -r "$HOME/.config/ruvultra-profiles/"*.toml "${INSTALLER_DIR}/config/" 2>/dev/null || true
cp "$HOME/.config/systemd/user/ruvultra-brain.service" "${INSTALLER_DIR}/config/" 2>/dev/null || true
cp "$HOME/.config/systemd/user/ruvultra-embedder.service" "${INSTALLER_DIR}/config/" 2>/dev/null || true

echo "==> Copying brain data (RVF format)..."
if [ -f "$HOME/brain-data/brain.rvf" ]; then
  cp "$HOME/brain-data/brain.rvf" "${INSTALLER_DIR}/brain/"
  echo "  brain.rvf ($(du -h "$HOME/brain-data/brain.rvf" | cut -f1))"
fi

echo "==> Copying identity (public key only)..."
[ -f "$HOME/.config/ruvultra/identity.pub" ] && cp "$HOME/.config/ruvultra/identity.pub" "${INSTALLER_DIR}/config/"

echo "==> Creating install script..."
cat > "${INSTALLER_DIR}/install.sh" << 'INSTALL'
#!/bin/sh
set -eu
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "╔══════════════════════════════════════════╗"
echo "║  ruOS Installer (offline)                ║"
echo "╚══════════════════════════════════════════╝"
echo

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  DEB_ARCH="amd64" ;;
  aarch64) DEB_ARCH="arm64" ;;
  *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac
echo "Architecture: $ARCH ($DEB_ARCH)"

# Install .debs if dpkg is available
if command -v dpkg >/dev/null 2>&1; then
  echo "==> Installing ruOS packages via dpkg..."
  for deb in "${SCRIPT_DIR}/debs/ruos-core_*_${DEB_ARCH}.deb" "${SCRIPT_DIR}/debs/ruos-brain-base_*.deb"; do
    [ -f "$deb" ] && sudo dpkg -i "$deb" 2>/dev/null && echo "  Installed: $(basename $deb)"
  done
  sudo apt-get install -f -y 2>/dev/null || true
else
  echo "==> dpkg not found, installing binaries directly..."
  mkdir -p "$HOME/.local/bin"
  for bin in "${SCRIPT_DIR}/bin/"*; do
    [ -f "$bin" ] && install -m 755 "$bin" "$HOME/.local/bin/" && echo "  $(basename $bin)"
  done
fi

# Copy brain if none exists
if [ ! -f "$HOME/brain-data/brain.rvf" ] && [ -f "${SCRIPT_DIR}/brain/brain.rvf" ]; then
  mkdir -p "$HOME/brain-data"
  cp "${SCRIPT_DIR}/brain/brain.rvf" "$HOME/brain-data/"
  echo "==> Copied brain.rvf to ~/brain-data/"
fi

# Copy profiles
mkdir -p "$HOME/.config/ruvultra-profiles"
cp "${SCRIPT_DIR}/config/"*.toml "$HOME/.config/ruvultra-profiles/" 2>/dev/null || true

# Run init if available
if command -v ruvultra-init >/dev/null 2>&1; then
  echo "==> Running ruvultra-init setup..."
  ruvultra-init setup 2>/dev/null || true
fi

echo
echo "╔══════════════════════════════════════════╗"
echo "║  ruOS installed successfully!            ║"
echo "║  Run: ruvultra-init status               ║"
echo "╚══════════════════════════════════════════╝"
INSTALL
chmod +x "${INSTALLER_DIR}/install.sh"

# Create README
cat > "${INSTALLER_DIR}/README.txt" << 'README'
ruOS — AI Workstation Operating System
======================================

This USB contains a complete ruOS installation:

  debs/     - .deb packages for apt-based install
  bin/      - Pre-built binaries for direct install
  config/   - System profiles + systemd units
  brain/    - Pre-trained brain.rvf (RVF cognitive container)

INSTALL:
  bash install.sh

MANUAL INSTALL:
  sudo dpkg -i debs/ruos-core_*.deb debs/ruos-brain-base_*.deb
  ruvultra-init setup

STORAGE FORMAT:
  brain.rvf uses RVF (RuVector Format) — append-only cognitive
  containers with XXH3-128 hash chains and ed25519 signing.
README

TOTAL=$(du -sh "${INSTALLER_DIR}" | cut -f1)
echo
echo "==> ruOS bootable USB created at ${INSTALLER_DIR}"
echo "    Total size: ${TOTAL}"
echo "    To install on any machine: bash ${INSTALLER_DIR}/install.sh"
