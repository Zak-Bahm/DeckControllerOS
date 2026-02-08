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

CURRENT_HASH="$(sha256sum "$OUT_DIR/.config" | awk '{print $1}')"
PREVIOUS_HASH=""
if [ -f "$CONFIG_HASH_FILE" ]; then
  PREVIOUS_HASH="$(cat "$CONFIG_HASH_FILE")"
fi

NEEDS_CLEAN_REBUILD=0
if [ -z "$PREVIOUS_HASH" ] && [ -d "$OUT_DIR/build" ]; then
  NEEDS_CLEAN_REBUILD=1
elif [ -n "$PREVIOUS_HASH" ] && [ "$PREVIOUS_HASH" != "$CURRENT_HASH" ]; then
  NEEDS_CLEAN_REBUILD=1
fi

if [ "$NEEDS_CLEAN_REBUILD" -eq 1 ]; then
  echo "Buildroot config changed; running clean rebuild to avoid stale package configuration"
  make -C "$BUILDROOT_DIR" \
    O="$OUT_DIR" \
    BR2_EXTERNAL="$BR2_EXTERNAL_DIR" clean
  make -C "$BUILDROOT_DIR" \
    O="$OUT_DIR" \
    BR2_EXTERNAL="$BR2_EXTERNAL_DIR" \
    BR2_DEFCONFIG="$DEFCONFIG" defconfig
  CURRENT_HASH="$(sha256sum "$OUT_DIR/.config" | awk '{print $1}')"
fi

echo "$CURRENT_HASH" > "$CONFIG_HASH_FILE"

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
