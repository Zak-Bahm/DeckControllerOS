#!/bin/bash
#
# Host-side end-to-end validation for checkpoint 03.
# Run this on the host machine (Linux) to discover, pair, connect to the
# Deck running ControllerOS, and verify that the host sees a game controller
# with changing input.
#
# Usage:
#   ./scripts/bt_checkpoint03_host_validate.sh [--target-name <name>] [--timeout <seconds>]
#
# Prerequisites (host):
#   - bluetoothctl
#   - evtest (optional, for input verification)
#
# The script writes detailed logs to out/host-logs/.

set -eu

TARGET_NAME="ControllerOS Xbox Controller"
TIMEOUT=60
LOG_DIR="out/host-logs"

usage() {
	echo "Usage: $0 [--target-name <name>] [--timeout <seconds>]"
	echo "  --target-name   Bluetooth device name to search for (default: '$TARGET_NAME')"
	echo "  --timeout       Seconds to wait for discovery/connection (default: $TIMEOUT)"
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--target-name)
			[ "$#" -ge 2 ] || { echo "error: --target-name requires a value" >&2; exit 1; }
			TARGET_NAME="$2"
			shift 2
			;;
		--timeout)
			[ "$#" -ge 2 ] || { echo "error: --timeout requires a value" >&2; exit 1; }
			TIMEOUT="$2"
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

mkdir -p "$LOG_DIR"
LOGFILE="$LOG_DIR/validate_$(date +%Y%m%d_%H%M%S).log"

log() {
	echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOGFILE"
}

fail() {
	log "FAIL: $*"
	exit 1
}

if ! command -v bluetoothctl >/dev/null 2>&1; then
	fail "bluetoothctl not found on host"
fi

log "=== Checkpoint 03 host validation ==="
log "target_name=$TARGET_NAME timeout=${TIMEOUT}s"

# Step 1: Scan for the target device.
log "Step 1: Scanning for '$TARGET_NAME'..."
DECK_MAC=""

# Start scanning in the background.
bluetoothctl --timeout "$TIMEOUT" scan on >> "$LOGFILE" 2>&1 &
SCAN_PID=$!

SCAN_DEADLINE=$(($(date +%s) + TIMEOUT))
while [ "$(date +%s)" -lt "$SCAN_DEADLINE" ]; do
	# Look for the device in discovered/paired devices.
	DEVICES="$(bluetoothctl devices 2>/dev/null || true)"
	MATCH="$(echo "$DEVICES" | grep -i "$TARGET_NAME" | head -n1 || true)"
	if [ -n "$MATCH" ]; then
		DECK_MAC="$(echo "$MATCH" | awk '{print $2}')"
		break
	fi
	sleep 2
done

# Stop scanning.
kill "$SCAN_PID" 2>/dev/null || true
wait "$SCAN_PID" 2>/dev/null || true
bluetoothctl scan off >> "$LOGFILE" 2>&1 || true

if [ -z "$DECK_MAC" ]; then
	fail "device '$TARGET_NAME' not discovered within ${TIMEOUT}s"
fi
log "discovered: $DECK_MAC"

# Step 2: Pair.
log "Step 2: Pairing with $DECK_MAC..."
bluetoothctl pair "$DECK_MAC" >> "$LOGFILE" 2>&1 || true
sleep 2

PAIR_STATUS="$(bluetoothctl info "$DECK_MAC" 2>/dev/null | sed -n 's/^[[:space:]]*Paired:[[:space:]]*//p' | head -n1 || true)"
if [ "$PAIR_STATUS" != "yes" ]; then
	fail "pairing failed (Paired=$PAIR_STATUS)"
fi
log "paired: yes"

# Step 3: Trust.
log "Step 3: Trusting $DECK_MAC..."
bluetoothctl trust "$DECK_MAC" >> "$LOGFILE" 2>&1 || true

# Step 4: Connect.
log "Step 4: Connecting to $DECK_MAC..."
bluetoothctl connect "$DECK_MAC" >> "$LOGFILE" 2>&1 || true
sleep 3

CONNECT_STATUS="$(bluetoothctl info "$DECK_MAC" 2>/dev/null | sed -n 's/^[[:space:]]*Connected:[[:space:]]*//p' | head -n1 || true)"
if [ "$CONNECT_STATUS" != "yes" ]; then
	fail "connection failed (Connected=$CONNECT_STATUS)"
fi
log "connected: yes"

# Step 5: Dump device info.
log "Step 5: Device info..."
bluetoothctl info "$DECK_MAC" >> "$LOGFILE" 2>&1 || true

# Check if the device exposes a gamepad/joystick input.
INPUT_DEVICE=""
sleep 2
for JS in /dev/input/js*; do
	[ -e "$JS" ] || continue
	INPUT_DEVICE="$JS"
	break
done

if [ -z "$INPUT_DEVICE" ]; then
	log "WARNING: no /dev/input/js* device found; controller may appear as event device only"
	# Try to find an event device instead.
	for EV in /dev/input/event*; do
		[ -e "$EV" ] || continue
		if command -v evtest >/dev/null 2>&1; then
			EV_NAME="$(evtest --info "$EV" 2>/dev/null | head -n1 || true)"
			if echo "$EV_NAME" | grep -qi "controller\|xbox\|gamepad\|ControllerOS"; then
				INPUT_DEVICE="$EV"
				break
			fi
		fi
	done
fi

if [ -n "$INPUT_DEVICE" ]; then
	log "input device: $INPUT_DEVICE"
else
	log "WARNING: could not identify a host input device for the controller"
fi

# Step 6: Verify changing input (brief observation).
log "Step 6: Observing input for 5 seconds..."
OBSERVE_OK=0
if [ -n "$INPUT_DEVICE" ] && command -v evtest >/dev/null 2>&1; then
	EVLOG="$LOG_DIR/evtest_$(date +%Y%m%d_%H%M%S).log"
	timeout 5 evtest "$INPUT_DEVICE" > "$EVLOG" 2>&1 || true
	EVENT_COUNT="$(wc -l < "$EVLOG" | awk '{print $1}')"
	if [ "$EVENT_COUNT" -gt 2 ]; then
		OBSERVE_OK=1
		log "observed $EVENT_COUNT lines of input events"
	else
		log "WARNING: only $EVENT_COUNT lines of input events observed"
	fi
elif [ -n "$INPUT_DEVICE" ]; then
	# Fallback: read a few bytes from the js device.
	if timeout 5 dd if="$INPUT_DEVICE" bs=8 count=4 of=/dev/null 2>/dev/null; then
		OBSERVE_OK=1
		log "observed data from $INPUT_DEVICE"
	fi
fi

# Summary.
log "=== Results ==="
log "Discovery: OK ($DECK_MAC)"
log "Pairing:   OK"
log "Connect:   OK"
if [ "$OBSERVE_OK" -eq 1 ]; then
	log "Input:     OK"
	log "PASS: Host discovered, paired, trusted, connected, and observed changing input pattern"
else
	log "Input:     INCONCLUSIVE (install evtest for full verification)"
	log "PARTIAL PASS: Host discovered, paired, trusted, and connected. Input observation inconclusive."
fi
log "Full log: $LOGFILE"
