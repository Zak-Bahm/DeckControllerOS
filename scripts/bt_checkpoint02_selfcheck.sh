#!/bin/sh

set -eu

REQUIRE_PAIRED=0
EXPECT_MAC=""

usage() {
	echo "Usage: $0 [--require-paired] [--expect-mac <MAC>]"
	echo "  --require-paired      fail if no paired devices are present"
	echo "  --expect-mac <MAC>    fail unless this MAC is in paired devices"
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--require-paired)
			REQUIRE_PAIRED=1
			shift
			;;
		--expect-mac)
			[ "$#" -ge 2 ] || {
				echo "error: --expect-mac requires a value" >&2
				usage >&2
				exit 1
			}
			EXPECT_MAC="$2"
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

if ! command -v bluetoothctl >/dev/null 2>&1; then
	echo "error: bluetoothctl not found in PATH" >&2
	exit 1
fi

if ! command -v mountpoint >/dev/null 2>&1; then
	echo "error: mountpoint command not found in PATH" >&2
	exit 1
fi

if ! SHOW_OUTPUT="$(bluetoothctl show 2>/dev/null)"; then
	echo "error: no Bluetooth controller available" >&2
	exit 1
fi

extract_state() {
	key="$1"
	printf '%s\n' "$SHOW_OUTPUT" | sed -n "s/^[[:space:]]*$key:[[:space:]]*//p" | head -n1
}

POWERED_STATE="$(extract_state Powered)"
PAIRABLE_STATE="$(extract_state Pairable)"
DISCOVERABLE_STATE="$(extract_state Discoverable)"

if [ "$POWERED_STATE" != "yes" ]; then
	echo "error: Powered state is '$POWERED_STATE' (expected 'yes')" >&2
	exit 1
fi
if [ "$PAIRABLE_STATE" != "yes" ]; then
	echo "error: Pairable state is '$PAIRABLE_STATE' (expected 'yes')" >&2
	exit 1
fi
if [ "$DISCOVERABLE_STATE" != "yes" ]; then
	echo "error: Discoverable state is '$DISCOVERABLE_STATE' (expected 'yes')" >&2
	exit 1
fi

if ! mountpoint -q /var/lib/controlleros; then
	echo "error: /var/lib/controlleros is not mounted" >&2
	exit 1
fi

if ! mountpoint -q /var/lib/bluetooth; then
	echo "error: /var/lib/bluetooth is not mounted" >&2
	exit 1
fi

PAIRED_LIST="$(bluetoothctl devices Paired)"
PAIRED_COUNT="$(printf '%s\n' "$PAIRED_LIST" | sed '/^[[:space:]]*$/d' | wc -l | awk '{print $1}')"

if [ "$REQUIRE_PAIRED" -eq 1 ] && [ "$PAIRED_COUNT" -eq 0 ]; then
	echo "error: no paired devices found" >&2
	exit 1
fi

if [ -n "$EXPECT_MAC" ] && ! printf '%s\n' "$PAIRED_LIST" | grep -qi "$EXPECT_MAC"; then
	echo "error: expected paired MAC '$EXPECT_MAC' not found" >&2
	exit 1
fi

echo "Checkpoint 02 self-check: PASS"
echo "Powered=$POWERED_STATE Pairable=$PAIRABLE_STATE Discoverable=$DISCOVERABLE_STATE"
echo "PairedDevices=$PAIRED_COUNT"
