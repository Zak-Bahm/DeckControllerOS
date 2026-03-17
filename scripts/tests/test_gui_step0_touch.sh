#!/bin/sh
# Test script for GUI Step 0: Manual touchscreen verification
# Reads raw input events from the touchscreen for 10 seconds.
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --timeout-seconds 20 --shell-script test_gui_step0_touch.sh

# Send a live message to the dev-payload server via POST /send-instruction.
# Uses CONTROLLEROS_DEV_BASE_URL exported by controlleros-dev-run.
live_msg() {
    MSG="$1"
    if [ -z "${CONTROLLEROS_DEV_BASE_URL:-}" ]; then
        return
    fi
    TMP_MSG="$(mktemp /tmp/live_msg.XXXXXX)"
    printf '%s' "$MSG" > "$TMP_MSG"
    if command -v curl >/dev/null 2>&1; then
        curl -fsS -X POST -H "Content-Type: text/plain" \
            --data-binary @"$TMP_MSG" \
            "$CONTROLLEROS_DEV_BASE_URL/send-instruction" >/dev/null 2>&1 || true
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O /dev/null --post-file="$TMP_MSG" \
            "$CONTROLLEROS_DEV_BASE_URL/send-instruction" 2>/dev/null || true
    fi
    rm -f "$TMP_MSG"
}

# Find the touchscreen event device
TOUCH_EVENT=""
for dev in /sys/class/input/event*; do
    [ -e "$dev" ] || continue
    NAME_FILE="$dev/device/name"
    [ -f "$NAME_FILE" ] || continue
    NAME="$(cat "$NAME_FILE")"
    case "$NAME" in
        *FTS3528*|*[Tt]ouch*|*[Mm]ultitouch*)
            TOUCH_EVENT="/dev/input/$(basename "$dev")"
            echo "Found touchscreen: $NAME -> $TOUCH_EVENT"
            break
            ;;
    esac
done

if [ -z "$TOUCH_EVENT" ]; then
    echo "FAIL: No touchscreen event device found"
    exit 1
fi

if [ ! -r "$TOUCH_EVENT" ]; then
    echo "FAIL: Cannot read $TOUCH_EVENT"
    exit 1
fi

DURATION=10
TMP_DATA="/tmp/touch_test_data"
rm -f "$TMP_DATA"

live_msg "TAP THE TOUCHSCREEN NOW — recording for ${DURATION}s from $TOUCH_EVENT"
echo "Recording touch input from $TOUCH_EVENT for ${DURATION} seconds..."
echo "TAP THE TOUCHSCREEN NOW"

dd if="$TOUCH_EVENT" of="$TMP_DATA" bs=64 2>/dev/null &
DD_PID=$!

sleep "$DURATION"
kill "$DD_PID" 2>/dev/null
wait "$DD_PID" 2>/dev/null

BYTES=0
[ -f "$TMP_DATA" ] && BYTES=$(wc -c < "$TMP_DATA")
echo "Received $BYTES bytes from $TOUCH_EVENT"

if [ "$BYTES" -gt 0 ]; then
    echo "PASS: Touchscreen produced $BYTES bytes of input data"
    echo ""
    echo "First 128 bytes (hex):"
    od -A x -t x1z -N 128 "$TMP_DATA" 2>/dev/null || hexdump -C -n 128 "$TMP_DATA" 2>/dev/null || echo "(no hex dump tool)"
    live_msg "PASS: Touchscreen produced $BYTES bytes of input data"
else
    echo "FAIL: No data received — touchscreen may not be responding to touch"
    live_msg "FAIL: No touch data received"
fi

rm -f "$TMP_DATA"

if [ "$BYTES" -gt 0 ]; then
    exit 0
else
    exit 1
fi
