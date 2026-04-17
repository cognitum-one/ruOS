#!/bin/sh
set -eu

VERSION="1.1.0"
PKG="ruos-agent"
ARCH="all"
DEB_DIR="$(cd "$(dirname "$0")/.." && pwd)/out/deb"
STAGE="$(mktemp -d)"
SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building ${PKG}_${VERSION}_${ARCH}.deb"

mkdir -p "${STAGE}/DEBIAN"
mkdir -p "${STAGE}/usr/local/bin"
mkdir -p "${STAGE}/usr/local/lib/ruos"
mkdir -p "${STAGE}/usr/lib/systemd/user"

# Copy agent binaries
for f in ruos-agent ruos-llm-serve ruos-update; do
    src="${SCRIPT_DIR}/packages/ruos-agent/$f"
    [ -f "$src" ] && install -m 755 "$src" "${STAGE}/usr/local/bin/"
done

# Copy AIDefence module
[ -f "${SCRIPT_DIR}/packages/ruos-agent/aidefence.py" ] && \
    cp "${SCRIPT_DIR}/packages/ruos-agent/aidefence.py" "${STAGE}/usr/local/lib/ruos/"

# Copy bootstrap
[ -f "${SCRIPT_DIR}/scripts/ruos-bootstrap" ] && \
    install -m 755 "${SCRIPT_DIR}/scripts/ruos-bootstrap" "${STAGE}/usr/local/bin/"

# Copy systemd units
for unit in ruos-agent.service ruos-agent.timer ruos-agent-nightly.service \
            ruos-agent-nightly.timer ruos-llm.service ruos-update.service \
            ruos-update.timer; do
    src="${SCRIPT_DIR}/config/systemd/$unit"
    [ -f "$src" ] && cp "$src" "${STAGE}/usr/lib/systemd/user/"
done

# Control file
cat > "${STAGE}/DEBIAN/control" << EOF
Package: ${PKG}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: ruv <ruv@ruv.net>
Depends: python3 (>= 3.10), ruos-core
Recommends: ruos-embedder | ruos-embedder-intel
Suggests: python3-torch, python3-transformers
Description: ruOS agentic daemon — autonomous observe-reason-act loop
 The ruos-agent daemon runs every 5 minutes to monitor services,
 auto-switch GPU profiles via LLM reasoning (Qwen2.5-3B), backfill
 embeddings, run nightly DPO training, evaluate search quality,
 and check for OTA updates. Includes AIDefence security layer,
 ruos-bootstrap wizard, and ruos-llm-serve inference server.
Homepage: https://github.com/cognitum-one/ruVultra-linux
EOF

# Post-install: enable timers
cat > "${STAGE}/DEBIAN/postinst" << 'POSTINST'
#!/bin/sh
set -e
if [ -d "$HOME/.config/systemd/user" ] 2>/dev/null; then
    # Copy units to user systemd if not using system-wide
    for unit in /usr/lib/systemd/user/ruos-*.service /usr/lib/systemd/user/ruos-*.timer; do
        [ -f "$unit" ] && cp "$unit" "$HOME/.config/systemd/user/" 2>/dev/null || true
    done
    systemctl --user daemon-reload 2>/dev/null || true
    systemctl --user enable ruos-agent.timer 2>/dev/null || true
    systemctl --user enable ruos-agent-nightly.timer 2>/dev/null || true
    systemctl --user enable ruos-update.timer 2>/dev/null || true
    echo "ruos-agent timers enabled. Start with: systemctl --user start ruos-agent.timer"
fi
# Create lib dir
mkdir -p /usr/local/lib/ruos 2>/dev/null || true
POSTINST
chmod 755 "${STAGE}/DEBIAN/postinst"

mkdir -p "$DEB_DIR"
dpkg-deb --build "$STAGE" "${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb"
SIZE=$(stat -c%s "${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb")
echo "==> Built: ${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb (${SIZE} bytes)"

rm -rf "$STAGE"
