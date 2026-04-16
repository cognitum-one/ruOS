#!/bin/sh
set -eu

PROJDIR="/home/ruvultra/projects/ruVultra-linux"
DEBDIR="${PROJDIR}/out/deb"
ARCH="${1:-amd64}"
DEB="ruvultra-core_0.7.0_${ARCH}.deb"

if [ ! -f "${DEBDIR}/${DEB}" ]; then
  echo "ERROR: ${DEBDIR}/${DEB} not found — run 'make deb-${ARCH}' first" >&2
  exit 1
fi

echo "==> Testing ${DEB} in Docker (ubuntu:24.04)"

CONTAINER="ruvultra-test-install-$$"

cleanup() {
  docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "${CONTAINER}" ubuntu:24.04 sleep 300

docker cp "${DEBDIR}/${DEB}" "${CONTAINER}:/tmp/${DEB}"

docker exec "${CONTAINER}" sh -c "
  set -e
  apt-get update -qq
  apt-get install -y -qq sqlite3 ca-certificates >/dev/null 2>&1
  dpkg -i /tmp/${DEB}
  echo '--- Verifying binaries ---'
  ruvultra-mcp --help
  echo '---'
  ruvultra-profile --help || ruvultra-profile --version || true
  echo '--- Checking file layout ---'
  test -f /usr/local/bin/ruvultra-mcp
  test -f /usr/local/sbin/ruvultra-profile
  test -d /etc/ruvultra-profiles
  test -f /etc/sudoers.d/ruvultra-profile
  echo 'ALL CHECKS PASSED'
"

RESULT=$?

if [ "${RESULT}" -eq 0 ]; then
  echo "==> Test PASSED: .deb installs cleanly on Ubuntu 24.04"
  exit 0
else
  echo "==> Test FAILED" >&2
  exit 1
fi
