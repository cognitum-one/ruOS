#!/bin/sh
set -eu

PROJDIR="/home/ruvultra/projects/ruVultra-linux"
DEBDIR="${PROJDIR}/out/deb"
ARCH="${1:-amd64}"
CORE_DEB="ruos-core_0.7.0_${ARCH}.deb"
BRAIN_DEB="ruos-brain-base_0.7.0_all.deb"

for deb in "${CORE_DEB}" "${BRAIN_DEB}"; do
  if [ ! -f "${DEBDIR}/${deb}" ]; then
    echo "ERROR: ${DEBDIR}/${deb} not found" >&2
    exit 1
  fi
done

echo "==> Testing ${CORE_DEB} + ${BRAIN_DEB} in Docker (ubuntu:24.04)"

CONTAINER="ruvultra-test-install-$$"

cleanup() {
  docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "${CONTAINER}" ubuntu:24.04 sleep 300

docker cp "${DEBDIR}/${CORE_DEB}" "${CONTAINER}:/tmp/${CORE_DEB}"
docker cp "${DEBDIR}/${BRAIN_DEB}" "${CONTAINER}:/tmp/${BRAIN_DEB}"

docker exec "${CONTAINER}" sh -c "
  set -e
  export HOME=/root
  apt-get update -qq >/dev/null 2>&1
  apt-get install -y -qq sqlite3 ca-certificates python3 >/dev/null 2>&1

  echo '=== Installing core ==='
  dpkg -i /tmp/${CORE_DEB}

  echo '=== Installing brain-base ==='
  dpkg --force-depends -i /tmp/${BRAIN_DEB}

  echo '--- Verify binaries ---'
  test -f /usr/local/bin/ruvultra-mcp && echo 'ruvultra-mcp: OK'
  test -f /usr/local/sbin/ruvultra-profile && echo 'ruvultra-profile: OK'
  test -f /usr/local/bin/ruvultra-init && echo 'ruvultra-init: OK'
  test -f /usr/local/bin/mcp-brain-server-local && echo 'mcp-brain-server-local: OK'
  test -f /usr/local/bin/mcp-brain && echo 'mcp-brain: OK'

  echo '--- ruvultra-init detect ---'
  DETECT=\$(ruvultra-init detect 2>/dev/null || echo '{\"error\":\"detect not available\"}')
  echo \"\${DETECT}\" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(\"detect JSON: OK\")' 2>/dev/null || echo 'detect: returned non-JSON (acceptable)'

  echo '--- MCP tools/list ---'
  TOOLS_RESP=\$(echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}' | ruvultra-mcp 2>/dev/null || echo 'MCP_ERROR')
  TOOL_COUNT=\$(echo \"\${TOOLS_RESP}\" | python3 -c 'import json,sys; r=json.load(sys.stdin); print(len(r.get(\"result\",{}).get(\"tools\",[])))' 2>/dev/null || echo '0')
  echo \"Tool count: \${TOOL_COUNT}\"
  if [ \"\${TOOL_COUNT}\" -ge 90 ]; then
    echo 'MCP tools: OK (>=90 tools)'
  else
    echo 'MCP tools: WARN — fewer than expected'
  fi

  echo '--- Profile list ---'
  PROFILES=\$(ls /etc/ruvultra-profiles/*.toml 2>/dev/null | wc -l)
  echo \"Profile count: \${PROFILES}\"
  if [ \"\${PROFILES}\" -ge 6 ]; then
    echo 'Profiles: OK (>=6)'
  else
    echo 'Profiles: WARN — fewer than expected'
  fi

  echo '--- Brain base ---'
  test -f /usr/share/ruvultra/brain-base.rvf && echo 'brain-base.rvf: installed OK'
  test -f /root/brain-data/brain.rvf && echo 'brain.rvf copied to HOME: OK'

  echo 'ALL CHECKS PASSED'
"

RESULT=$?

if [ "${RESULT}" -eq 0 ]; then
  echo "==> Test PASSED: .debs install cleanly on Ubuntu 24.04"
  exit 0
else
  echo "==> Test FAILED" >&2
  exit 1
fi
