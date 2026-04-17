#!/bin/sh
set -eu

VERSION="1.1.0"
PKG="ruos-embedder-intel"
ARCH="all"
DEB_DIR="$(cd "$(dirname "$0")/.." && pwd)/out/deb"
STAGE="$(mktemp -d)"
SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building ${PKG}_${VERSION}_${ARCH}.deb"

mkdir -p "${STAGE}/DEBIAN"
mkdir -p "${STAGE}/usr/local/bin"
mkdir -p "${STAGE}/usr/local/lib/ruos-embedder-intel"

# Copy the embedder script
cp "${SCRIPT_DIR}/packages/ruos-embedder-intel/ruvultra-embedder-intel" "${STAGE}/usr/local/bin/"
chmod 755 "${STAGE}/usr/local/bin/ruvultra-embedder-intel"

# Control file
cat > "${STAGE}/DEBIAN/control" << EOF
Package: ${PKG}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: ruv <ruv@ruv.net>
Depends: python3 (>= 3.10)
Recommends: python3-pip
Suggests: intel-opencl-icd
Conflicts: ruos-embedder
Description: ruOS Intel embedding service — OpenVINO-accelerated bge-small-en-v1.5
 Drop-in replacement for the CUDA embedder that runs on Intel hardware:
 Intel CPU (AVX-512/AMX), Intel Arc GPU, or Intel iGPU. Same API, same
 model (384-d vectors), different backend. Requires one of:
 optimum[openvino], onnxruntime-openvino, or sentence-transformers.
Homepage: https://github.com/cognitum-one/ruVultra-linux
EOF

# Post-install: suggest pip install
cat > "${STAGE}/DEBIAN/postinst" << 'POSTINST'
#!/bin/sh
echo "ruos-embedder-intel installed."
echo "Install ML backend (pick one):"
echo "  pip install optimum[openvino] transformers    # Best: OpenVINO native"
echo "  pip install onnxruntime-openvino transformers  # Alt: ONNX + OpenVINO EP"
echo "  pip install sentence-transformers              # Fallback: PyTorch CPU"
POSTINST
chmod 755 "${STAGE}/DEBIAN/postinst"

mkdir -p "$DEB_DIR"
dpkg-deb --build "$STAGE" "${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb"
SIZE=$(stat -c%s "${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb")
echo "==> Built: ${DEB_DIR}/${PKG}_${VERSION}_${ARCH}.deb (${SIZE} bytes)"

rm -rf "$STAGE"
