#!/bin/sh

set -eu

OUT_DIR="out/dev-payload"
HIDD_BIN=""
CTL_BIN=""
HID_CONFIG=""

usage() {
	echo "Usage: $0 --hidd <path> --controllerosctl <path> [options]"
	echo "Options:"
	echo "  --hidd <path>             Path to hidd binary"
	echo "  --controllerosctl <path>  Path to controllerosctl binary"
	echo "  --hid-config <path>       Optional hid.toml path"
	echo "  --out-dir <path>          Output directory (default: out/dev-payload)"
	echo "  -h, --help                Show help"
}

copy_item() {
	SRC="$1"
	REL="$2"
	MODE="$3"
	DST="$OUT_DIR/$REL"
	DST_DIR="$(dirname "$DST")"

	[ -f "$SRC" ] || {
		echo "error: source file not found: $SRC" >&2
		exit 1
	}

	mkdir -p "$DST_DIR"
	cp -f "$SRC" "$DST"
	chmod "$MODE" "$DST"
	SHA="$(sha256sum "$DST" | awk '{print $1}')"
	printf '%s %s %s\n' "$MODE" "$SHA" "$REL" >> "$OUT_DIR/manifest.txt"
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--hidd)
			[ "$#" -ge 2 ] || {
				echo "error: --hidd requires a value" >&2
				usage >&2
				exit 1
			}
			HIDD_BIN="$2"
			shift 2
			;;
		--controllerosctl)
			[ "$#" -ge 2 ] || {
				echo "error: --controllerosctl requires a value" >&2
				usage >&2
				exit 1
			}
			CTL_BIN="$2"
			shift 2
			;;
		--hid-config)
			[ "$#" -ge 2 ] || {
				echo "error: --hid-config requires a value" >&2
				usage >&2
				exit 1
			}
			HID_CONFIG="$2"
			shift 2
			;;
		--out-dir)
			[ "$#" -ge 2 ] || {
				echo "error: --out-dir requires a value" >&2
				usage >&2
				exit 1
			}
			OUT_DIR="$2"
			shift 2
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "error: unknown argument: $1" >&2
			usage >&2
			exit 1
			;;
	esac
done

[ -n "$HIDD_BIN" ] || {
	echo "error: --hidd is required" >&2
	usage >&2
	exit 1
}
[ -n "$CTL_BIN" ] || {
	echo "error: --controllerosctl is required" >&2
	usage >&2
	exit 1
}

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
: > "$OUT_DIR/manifest.txt"

copy_item "$HIDD_BIN" "bin/hidd" "0755"
copy_item "$CTL_BIN" "bin/controllerosctl" "0755"

if [ -n "$HID_CONFIG" ]; then
	copy_item "$HID_CONFIG" "configs/hid/hid.toml" "0644"
fi

echo "Payload staged at: $OUT_DIR"
echo "Manifest: $OUT_DIR/manifest.txt"
