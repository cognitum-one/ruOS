#!/bin/sh
set -eu

ARCH="${1:?usage: build-deb.sh <amd64|arm64>}"
VERSION="0.7.0"
PKG="ruos-core"
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

# Source of pre-built binaries (not cross-compiled)
LOCALBIN="/home/ruvultra/.local/bin"

# Verify core binaries exist
for bin in ruvultra-mcp ruvultra-profile; do
  if [ ! -f "${BINDIR}/${bin}" ]; then
    echo "ERROR: ${BINDIR}/${bin} not found — run 'make ${ARCH}' first" >&2
    exit 1
  fi
done

# ruvultra-init is required for both arches
if [ ! -f "${LOCALBIN}/ruvultra-init" ]; then
  echo "ERROR: ${LOCALBIN}/ruvultra-init not found" >&2
  exit 1
fi

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
mkdir -p "${STAGE}/usr/local/share/ruos"

# Install core binaries (both arches)
install -m 755 "${BINDIR}/ruvultra-mcp" "${STAGE}/usr/local/bin/ruvultra-mcp"
install -m 755 "${BINDIR}/ruvultra-profile" "${STAGE}/usr/local/sbin/ruvultra-profile"

# Install ruvultra-init (amd64 pre-built; arm64 uses same binary via cross or pre-built)
if [ "${ARCH}" = "amd64" ]; then
  install -m 755 "${LOCALBIN}/ruvultra-init" "${STAGE}/usr/local/bin/ruvultra-init"
elif [ -f "${BINDIR}/ruvultra-init" ]; then
  install -m 755 "${BINDIR}/ruvultra-init" "${STAGE}/usr/local/bin/ruvultra-init"
else
  # For arm64, copy from local bin (native build) as fallback
  install -m 755 "${LOCALBIN}/ruvultra-init" "${STAGE}/usr/local/bin/ruvultra-init"
fi

# Install brain binaries (amd64 only — depend on ruvector workspace)
if [ "${ARCH}" = "amd64" ]; then
  if [ -f "${LOCALBIN}/mcp-brain-server-local" ]; then
    install -m 755 "${LOCALBIN}/mcp-brain-server-local" "${STAGE}/usr/local/bin/mcp-brain-server-local"
  else
    echo "WARN: mcp-brain-server-local not found, skipping" >&2
  fi
  if [ -f "${LOCALBIN}/mcp-brain" ]; then
    install -m 755 "${LOCALBIN}/mcp-brain" "${STAGE}/usr/local/bin/mcp-brain"
  else
    echo "WARN: mcp-brain not found, skipping" >&2
  fi
fi

# Install profile TOMLs
if [ -d "${SRCDIR}/config/profiles" ]; then
  cp "${SRCDIR}/config/profiles/"*.toml "${STAGE}/etc/ruvultra-profiles/"
fi

# Install systemd units
for svc in ruvultra-brain.service ruos-embedder.service; do
  if [ -f "${SRCDIR}/config/systemd/${svc}" ]; then
    cp "${SRCDIR}/config/systemd/${svc}" "${STAGE}/usr/lib/systemd/user/${svc}"
  fi
done

# Install sudoers drop-in
if [ -f "${SRCDIR}/sudoers.d/ruvultra-profile" ]; then
  install -m 440 "${SRCDIR}/sudoers.d/ruvultra-profile" "${STAGE}/etc/sudoers.d/ruvultra-profile"
fi

# Install agentic config files
if [ -f "${PROJDIR}/config/mcp.json" ]; then
  cp "${PROJDIR}/config/mcp.json" "${STAGE}/etc/ruvultra/mcp.json"
else
  # Fallback: minimal mcp.json template
  cat > "${STAGE}/etc/ruvultra/mcp.json" <<'MCPEOF'
{
  "mcpServers": {
    "ruvultra": {
      "command": "/usr/local/bin/ruvultra-mcp",
      "args": [],
      "env": { "RUST_LOG": "warn" },
      "autoStart": true
    },
    "mcp-brain": {
      "command": "/usr/local/bin/mcp-brain",
      "args": [],
      "env": {
        "RUST_LOG": "warn",
        "RUVBRAIN_URL": "http://127.0.0.1:9876",
        "MCP_BRAIN_VOTER": "ruos-local"
      },
      "autoStart": true
    }
  }
}
MCPEOF
fi

# Install CLAUDE.md (agentic instructions for Claude Code)
if [ -f "${PROJDIR}/config/CLAUDE.md" ]; then
  cp "${PROJDIR}/config/CLAUDE.md" "${STAGE}/etc/ruvultra/CLAUDE.md"
fi

# Install Claude Code installer script
if [ -f "${PROJDIR}/scripts/install-claude-code.sh" ]; then
  install -m 755 "${PROJDIR}/scripts/install-claude-code.sh" "${STAGE}/usr/local/share/ruos/install-claude-code.sh"
fi

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
/etc/ruvultra/mcp.json
/etc/ruvultra/CLAUDE.md
EOF

# Write postinst
cat > "${STAGE}/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e

# Reload systemd if available
if command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload 2>/dev/null || true
fi

# Generate identity if ruvultra-init is available
if [ -x /usr/local/bin/ruvultra-init ]; then
  /usr/local/bin/ruvultra-init identity 2>/dev/null || true
fi

# Try to install Claude Code (gracefully fails offline)
if [ -f /usr/local/share/ruos/install-claude-code.sh ]; then
  sh /usr/local/share/ruos/install-claude-code.sh 2>/dev/null || true
fi

echo "ruos-core installed. Run 'ruvultra-init setup' to configure the agentic environment."
echo "Then type 'claude' to start."
POSTINST
chmod 755 "${STAGE}/DEBIAN/postinst"

# Build .deb
mkdir -p "${DEBDIR}"
dpkg-deb --root-owner-group --build "${STAGE}" "${DEBDIR}/${PKG}_${VERSION}_${ARCH}.deb"

# Cleanup staging
rm -rf "${STAGE}"

SIZE=$(stat --printf='%s' "${DEBDIR}/${PKG}_${VERSION}_${ARCH}.deb")
echo "==> Built: ${DEBDIR}/${PKG}_${VERSION}_${ARCH}.deb (${SIZE} bytes)"
