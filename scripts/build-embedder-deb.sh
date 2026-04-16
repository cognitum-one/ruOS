#!/bin/sh
set -eu

VERSION="0.7.0"
PKG="ruos-embedder"
PROJDIR="/home/ruvultra/projects/ruVultra-linux"
OUTDIR="${PROJDIR}/out"
DEBDIR="${OUTDIR}/deb"
STAGE="${OUTDIR}/_stage-embedder"
LOCALBIN="/home/ruvultra/.local/bin"
MODEL_CACHE="/home/ruvultra/.cache/ruvultra-embedder/models--BAAI--bge-small-en-v1.5"

# Verify embedder binary exists
if [ ! -f "${LOCALBIN}/ruvultra-embedder" ]; then
  echo "ERROR: ${LOCALBIN}/ruvultra-embedder not found" >&2
  exit 1
fi

# Verify model cache exists
if [ ! -d "${MODEL_CACHE}" ]; then
  echo "ERROR: ${MODEL_CACHE} not found — run the embedder once to download the model" >&2
  exit 1
fi

echo "==> Packaging ${PKG}_${VERSION}_amd64.deb"

rm -rf "${STAGE}"
mkdir -p "${STAGE}/DEBIAN"
mkdir -p "${STAGE}/usr/local/bin"
mkdir -p "${STAGE}/usr/share/ruvultra/models/bge-small-en-v1.5"

# Install embedder binary
install -m 755 "${LOCALBIN}/ruvultra-embedder" "${STAGE}/usr/local/bin/ruvultra-embedder"

# Install model files (preserve directory structure)
cp -a "${MODEL_CACHE}/." "${STAGE}/usr/share/ruvultra/models/bge-small-en-v1.5/"

# Remove lock files from the model cache
find "${STAGE}/usr/share/ruvultra/models/bge-small-en-v1.5/" -name '*.lock' -delete

cat > "${STAGE}/DEBIAN/control" <<EOF
Package: ${PKG}
Version: ${VERSION}
Architecture: amd64
Maintainer: ruv <ruv@ruv.net>
Description: ruvultra local GPU embedder with bge-small-en-v1.5 model weights
Depends: ruos-core
Homepage: https://github.com/cognitum-one/ruVultra
Section: utils
Priority: optional
Installed-Size: $(du -sk "${STAGE}" | cut -f1)
EOF

cat > "${STAGE}/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
# Create symlink so the embedder finds the model in its expected cache location
CACHE_DIR="${HOME}/.cache/ruvultra-embedder"
MODEL_SRC="/usr/share/ruvultra/models/bge-small-en-v1.5"
MODEL_DST="${CACHE_DIR}/models--BAAI--bge-small-en-v1.5"

if [ ! -e "${MODEL_DST}" ]; then
  mkdir -p "${CACHE_DIR}"
  ln -sf "${MODEL_SRC}" "${MODEL_DST}"
  echo "ruos-embedder: linked model cache to ${MODEL_DST}"
else
  echo "ruos-embedder: model cache already exists at ${MODEL_DST}"
fi
POSTINST
chmod 755 "${STAGE}/DEBIAN/postinst"

mkdir -p "${DEBDIR}"
dpkg-deb --root-owner-group --build "${STAGE}" "${DEBDIR}/${PKG}_${VERSION}_amd64.deb"

rm -rf "${STAGE}"

SIZE=$(stat --printf='%s' "${DEBDIR}/${PKG}_${VERSION}_amd64.deb")
echo "==> Built: ${DEBDIR}/${PKG}_${VERSION}_amd64.deb (${SIZE} bytes)"
