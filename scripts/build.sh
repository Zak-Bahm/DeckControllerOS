#!/bin/sh

set -e

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
BUILDROOT_DIR="$ROOT_DIR/buildroot"
OUT_DIR="$ROOT_DIR/out/buildroot"
BR2_EXTERNAL_DIR="$ROOT_DIR/br2-external"
DEFCONFIG="$ROOT_DIR/configs/buildroot/controlleros_defconfig"
OUTPUT_ISO="$ROOT_DIR/out/controlleros.iso"
CONFIG_HASH_FILE="$OUT_DIR/.last_defconfig_hash"

make -C "$BUILDROOT_DIR" \
  O="$OUT_DIR" \
  BR2_EXTERNAL="$BR2_EXTERNAL_DIR" \
  BR2_DEFCONFIG="$DEFCONFIG" defconfig

# Force rebuild of local Rust crates. Buildroot stamps are existence-based,
# not timestamp-based — a newer .stamp_rsynced does NOT trigger a rebuild
# if .stamp_built already exists. We remove .stamp_rsynced to force re-sync,
# then use Buildroot's <pkg>-rebuild target to clear build/install stamps.
for pkg in controlleros-hidd controllerosctl controlleros-gui; do
  stamp="$OUT_DIR/build/${pkg}-0.1.0/.stamp_rsynced"
  if [ -f "$stamp" ]; then
    rm -f "$stamp"
    make -C "$BUILDROOT_DIR" \
      O="$OUT_DIR" \
      BR2_EXTERNAL="$BR2_EXTERNAL_DIR" \
      "${pkg}-rebuild"
  fi
done

make -C "$BUILDROOT_DIR" \
  O="$OUT_DIR" \
  BR2_EXTERNAL="$BR2_EXTERNAL_DIR"

if [ -f "$OUT_DIR/images/rootfs.iso9660" ]; then
  cp "$OUT_DIR/images/rootfs.iso9660" "$OUTPUT_ISO"
elif [ -f "$OUT_DIR/images/rootfs.img" ]; then
  cp "$OUT_DIR/images/rootfs.img" "$ROOT_DIR/out/controlleros.img"
else
  echo "No ISO or IMG produced in $OUT_DIR/images" >&2
  exit 1
fi
