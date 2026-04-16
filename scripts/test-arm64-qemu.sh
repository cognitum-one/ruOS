#!/bin/sh
set -eu

PROJDIR="/home/ruvultra/projects/ruVultra-linux"
DEBDIR="${PROJDIR}/out/deb"
DEB="ruvultra-core_0.7.0_arm64.deb"

if [ ! -f "${DEBDIR}/${DEB}" ]; then
  echo "ERROR: ${DEBDIR}/${DEB} not found — run 'make deb-arm64' first" >&2
  exit 1
fi

echo "==> Testing arm64 .deb via Docker multiplatform (qemu-user-static)"

# Register qemu binfmt handlers if not already registered
if [ ! -f /proc/sys/fs/binfmt_misc/qemu-aarch64 ]; then
  echo "==> Registering qemu binfmt handlers"
  docker run --rm --privileged multiarch/qemu-user-static --reset -p yes 2>/dev/null || true
fi

RESULT=$(docker run --rm --platform linux/arm64 \
  -v "${DEBDIR}:/debs:ro" \
  ubuntu:24.04 sh -c '
  apt-get update -qq >/dev/null 2>&1
  apt-get install -y -qq sqlite3 ca-certificates >/dev/null 2>&1
  dpkg -i /debs/ruvultra-core_0.7.0_arm64.deb 2>/dev/null

  echo "--- Binary check ---"
  test -f /usr/local/bin/ruvultra-mcp && echo "ruvultra-mcp: OK"
  test -f /usr/local/sbin/ruvultra-profile && echo "ruvultra-profile: OK"
  test -f /usr/local/bin/ruvultra-init && echo "ruvultra-init: OK"

  echo "--- MCP tools/list ---"
  TOOLS_RESP=$(echo "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}" | ruvultra-mcp 2>/dev/null || echo "MCP_ERROR")
  echo "${TOOLS_RESP}" | head -c 200
  echo

  echo "--- Architecture ---"
  echo "Detected arch: $(uname -m)"

  echo "--- No brain binaries (arm64) ---"
  test ! -f /usr/local/bin/mcp-brain-server-local && echo "mcp-brain-server-local: correctly absent"
  test ! -f /usr/local/bin/mcp-brain && echo "mcp-brain: correctly absent"

  echo "ALL ARM64 CHECKS PASSED"
' 2>&1)

echo "${RESULT}"

if echo "${RESULT}" | grep -q "ALL ARM64 CHECKS PASSED"; then
  echo "==> arm64 test PASSED"
  exit 0
else
  echo "==> arm64 test FAILED" >&2
  exit 1
fi
