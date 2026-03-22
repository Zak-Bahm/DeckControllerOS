#!/bin/sh

set -e

CLEAN_RUST=0
for arg in "$@"; do
  case "$arg" in
    --clean-rust) CLEAN_RUST=1 ;;
    -h|--help)
      echo "Usage: $0 [--clean-rust]"
      echo "  --clean-rust  Force a full clean rebuild of all Rust packages"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
BUILDROOT_DIR="$ROOT_DIR/buildroot"
OUT_DIR="$ROOT_DIR/out/buildroot"
BR2_EXTERNAL_DIR="$ROOT_DIR/br2-external"
DEFCONFIG="$ROOT_DIR/configs/buildroot/controlleros_defconfig"
OUTPUT_ISO="$ROOT_DIR/out/controlleros.iso"
CONFIG_HASH_FILE="$OUT_DIR/.last_defconfig_hash"

RUST_PACKAGES="hidd controllerosctl controlleros-gui"

make -C "$BUILDROOT_DIR" \
  O="$OUT_DIR" \
  BR2_EXTERNAL="$BR2_EXTERNAL_DIR" \
  BR2_DEFCONFIG="$DEFCONFIG" defconfig

if [ "$CLEAN_RUST" -eq 1 ]; then
  # Full dirclean: removes the entire build directory for each Rust package,
  # forcing a complete re-rsync, re-vendor, and recompile from scratch.
  echo "Cleaning Rust packages: $RUST_PACKAGES"
  for pkg in $RUST_PACKAGES; do
    make -C "$BUILDROOT_DIR" \
      O="$OUT_DIR" \
      BR2_EXTERNAL="$BR2_EXTERNAL_DIR" \
      "${pkg}-dirclean" 2>/dev/null || true
  done
fi

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
