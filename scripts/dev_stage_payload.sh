#!/bin/sh

set -eu

OUT_DIR="out/dev-payload"
HIDD_BIN=""
CTL_BIN=""
GUI_BIN=""
HID_CONFIG=""
MAPPING_CONFIG=""

# Default paths (relative to repo root)
DEFAULT_HIDD="target/release/hidd"
DEFAULT_CTL="target/release/controllerosctl"
DEFAULT_GUI="target/release/controlleros-gui"
DEFAULT_HID_CONFIG="configs/hid/hid.toml"
DEFAULT_MAPPING_CONFIG="configs/mapping/xbox.toml"

usage() {
	echo "Usage: $0 [options]"
	echo "At least one binary (--hidd, --controllerosctl, or --gui) is required."
	echo "Flags accept an optional path; if omitted, the default is used."
	echo ""
	echo "Options:"
	echo "  --hidd [path]             Path to hidd binary (default: $DEFAULT_HIDD)"
	echo "  --controllerosctl [path]  Path to controllerosctl binary (default: $DEFAULT_CTL)"
	echo "  --gui [path]              Path to controlleros-gui binary (default: $DEFAULT_GUI)"
	echo "  --hid-config [path]       hid.toml config (default: $DEFAULT_HID_CONFIG)"
	echo "  --mapping-config [path]   xbox.toml mapping config (default: $DEFAULT_MAPPING_CONFIG)"
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

# Check if a value looks like a flag (starts with -) or is missing.
# Returns 0 (true) if the next arg is a usable path value.
has_value() {
	[ "$#" -ge 2 ] || return 1
	case "$2" in
		-*) return 1 ;;
	esac
	return 0
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--hidd)
			if has_value "$@"; then
				HIDD_BIN="$2"
				shift 2
			else
				HIDD_BIN="$DEFAULT_HIDD"
				shift
			fi
			;;
		--controllerosctl)
			if has_value "$@"; then
				CTL_BIN="$2"
				shift 2
			else
				CTL_BIN="$DEFAULT_CTL"
				shift
			fi
			;;
		--gui)
			if has_value "$@"; then
				GUI_BIN="$2"
				shift 2
			else
				GUI_BIN="$DEFAULT_GUI"
				shift
			fi
			;;
		--hid-config)
			if has_value "$@"; then
				HID_CONFIG="$2"
				shift 2
			else
				HID_CONFIG="$DEFAULT_HID_CONFIG"
				shift
			fi
			;;
		--mapping-config)
			if has_value "$@"; then
				MAPPING_CONFIG="$2"
				shift 2
			else
				MAPPING_CONFIG="$DEFAULT_MAPPING_CONFIG"
				shift
			fi
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

if [ -z "$HIDD_BIN" ] && [ -z "$CTL_BIN" ] && [ -z "$GUI_BIN" ]; then
	echo "error: at least one of --hidd, --controllerosctl, or --gui is required" >&2
	usage >&2
	exit 1
fi

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
: > "$OUT_DIR/manifest.txt"

if [ -n "$HIDD_BIN" ]; then
	copy_item "$HIDD_BIN" "bin/hidd" "0755"
fi

if [ -n "$CTL_BIN" ]; then
	copy_item "$CTL_BIN" "bin/controllerosctl" "0755"
fi

if [ -n "$GUI_BIN" ]; then
	copy_item "$GUI_BIN" "bin/controlleros-gui" "0755"
fi

if [ -n "$HID_CONFIG" ]; then
	copy_item "$HID_CONFIG" "configs/hid/hid.toml" "0644"
fi

if [ -n "$MAPPING_CONFIG" ]; then
	copy_item "$MAPPING_CONFIG" "configs/mapping/xbox.toml" "0644"
fi

# Copy test scripts into payload root so controlleros-dev-run --shell-script can fetch them.
TESTS_DIR="$(cd "$(dirname "$0")" && pwd)/tests"
if [ -d "$TESTS_DIR" ]; then
	TEST_COUNT=0
	for f in "$TESTS_DIR"/*.sh; do
		[ -f "$f" ] || continue
		cp -f "$f" "$OUT_DIR/$(basename "$f")"
		chmod 0755 "$OUT_DIR/$(basename "$f")"
		TEST_COUNT=$((TEST_COUNT + 1))
	done
	if [ "$TEST_COUNT" -gt 0 ]; then
		echo "Copied $TEST_COUNT test script(s) from $TESTS_DIR"
	fi
fi

echo "Payload staged at: $OUT_DIR"
echo "Manifest: $OUT_DIR/manifest.txt"
