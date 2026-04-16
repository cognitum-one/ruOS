#!/bin/sh
set -eu

ARCH="${1:?usage: build-deb.sh <amd64|arm64>}"
VERSION="0.7.0"
PKG="ruvultra-core"
PROJDIR="/home/ruvultra/projects/ruVultra-linux"
SRCDIR="/home/ruvultra/projects/ruvultra-cognitum"
OUTDIR="${PROJDIR}/out"
BINDIR="${OUTDIR}/${ARCH}"
DEBDIR="${OUTDIR}/deb"
STAGE="${OUTDIR}/_stage-${ARCH}"

# Validate architecture
case "${ARCH}" in
  amd64|arm64) ;;
  *) echo "ERROR: architecture must be amd64 or arm64" >&2; exit 1 ;;
esac

# Verify binaries exist
for bin in ruvultra-mcp ruvultra-profile; do
  if [ ! -f "${BINDIR}/${bin}" ]; then
    echo "ERROR: ${BINDIR}/${bin} not found — run 'make ${ARCH}' first" >&2
    exit 1
  fi
done

echo "==> Packaging ${PKG}_${VERSION}_${ARCH}.deb"

# Clean staging area
rm -rf "${STAGE}"

# Create directory structure
mkdir -p "${STAGE}/DEBIAN"
mkdir -p "${STAGE}/usr/local/bin"
mkdir -p "${STAGE}/usr/local/sbin"
mkdir -p "${STAGE}/etc/ruvultra-profiles"
mkdir -p "${STAGE}/usr/lib/systemd/user"
mkdir -p "${STAGE}/etc/sudoers.d"
mkdir -p "${STAGE}/etc/ruvultra"

# Install binaries
install -m 755 "${BINDIR}/ruvultra-mcp" "${STAGE}/usr/local/bin/ruvultra-mcp"
install -m 755 "${BINDIR}/ruvultra-profile" "${STAGE}/usr/local/sbin/ruvultra-profile"

# Install profile TOMLs
if [ -d "${SRCDIR}/config/profiles" ]; then
  cp "${SRCDIR}/config/profiles/"*.toml "${STAGE}/etc/ruvultra-profiles/"
fi

# Install systemd units
for svc in ruvultra-brain.service ruvultra-embedder.service; do
  if [ -f "${SRCDIR}/config/systemd/${svc}" ]; then
    cp "${SRCDIR}/config/systemd/${svc}" "${STAGE}/usr/lib/systemd/user/${svc}"
  fi
done

# Install sudoers drop-in
if [ -f "${SRCDIR}/sudoers.d/ruvultra-profile" ]; then
  install -m 440 "${SRCDIR}/sudoers.d/ruvultra-profile" "${STAGE}/etc/sudoers.d/ruvultra-profile"
fi

# Install .mcp.json template
cat > "${STAGE}/etc/ruvultra/mcp.json.template" <<'MCPEOF'
{
  "mcpServers": {
    "ruvultra": {
      "command": "/usr/local/bin/ruvultra-mcp",
      "args": []
    }
  }
}
MCPEOF

# Write DEBIAN/control — architecture-specific, NOT "any"
cat > "${STAGE}/DEBIAN/control" <<EOF
Package: ${PKG}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: ruv <ruv@ruv.net>
Description: ruvultra AI workstation core — MCP server, profile helper, brain backend
Depends: sqlite3, ca-certificates
Homepage: https://github.com/cognitum-one/ruVultra
Section: utils
Priority: optional
Installed-Size: $(du -sk "${STAGE}" | cut -f1)
EOF

# Write conffiles (config files preserved on upgrade)
cat > "${STAGE}/DEBIAN/conffiles" <<EOF
/etc/ruvultra-profiles/default.toml
/etc/sudoers.d/ruvultra-profile
/etc/ruvultra/mcp.json.template
EOF

# Write postinst
cat > "${STAGE}/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
# Reload systemd if available
if command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload 2>/dev/null || true
fi
echo "ruvultra-core installed. Run 'ruvultra-mcp --help' to verify."
POSTINST
chmod 755 "${STAGE}/DEBIAN/postinst"

# Build .deb
mkdir -p "${DEBDIR}"
dpkg-deb --root-owner-group --build "${STAGE}" "${DEBDIR}/${PKG}_${VERSION}_${ARCH}.deb"

# Cleanup staging
rm -rf "${STAGE}"

SIZE=$(stat --printf='%s' "${DEBDIR}/${PKG}_${VERSION}_${ARCH}.deb")
echo "==> Built: ${DEBDIR}/${PKG}_${VERSION}_${ARCH}.deb (${SIZE} bytes)"
