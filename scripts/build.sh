#!/bin/sh

set -e

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
BUILDROOT_DIR="$ROOT_DIR/buildroot"
OUT_DIR="$ROOT_DIR/out/buildroot"
BR2_EXTERNAL_DIR="$ROOT_DIR/br2-external"
DEFCONFIG="$ROOT_DIR/configs/buildroot/controlleros_defconfig"
OUTPUT_ISO="$ROOT_DIR/out/controlleros.iso"

make -C "$BUILDROOT_DIR" \
  O="$OUT_DIR" \
  BR2_EXTERNAL="$BR2_EXTERNAL_DIR" \
  BR2_DEFCONFIG="$DEFCONFIG" defconfig

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
