#!/bin/sh
# Test script for GUI Step 6: BlueZ backend wired to Slint UI
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step6_wired.sh

export RUST_LOG=debug

PASS_COUNT=0
FAIL_COUNT=0

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

pass() {
    echo "PASS: $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
    echo "FAIL: $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

live_msg "Starting test_gui_step6_wired.sh"

# Check 1: Locate controlleros-gui binary
GUI_BIN=""
if [ -x /tmp/dev-update/bin/controlleros-gui ]; then
    GUI_BIN="/tmp/dev-update/bin/controlleros-gui"
    pass "controlleros-gui found at /tmp/dev-update/bin/controlleros-gui"
elif [ -x /usr/bin/controlleros-gui ]; then
    GUI_BIN="/usr/bin/controlleros-gui"
    pass "controlleros-gui found at /usr/bin/controlleros-gui"
else
    fail "controlleros-gui binary not found"
    echo "=== Results ==="
    echo "PASS: $PASS_COUNT"
    echo "FAIL: $FAIL_COUNT"
    exit 1
fi

# Snapshot paired devices via bluetoothctl
BTCTL_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)

echo "=== bluetoothctl paired devices ==="
echo "$BTCTL_DEVICES"
echo "=== end bluetoothctl devices ==="

if [ -z "$BTCTL_DEVICES" ]; then
    fail "No paired Bluetooth devices — at least one is required for this test"
    echo "=== Results ==="
    echo "PASS: $PASS_COUNT"
    echo "FAIL: $FAIL_COUNT"
    exit 1
else
    pass "Paired Bluetooth devices found"
fi

# Kill tty1 getty to free the primary VT for GUI rendering
GETTY_PID=$(ps | grep '[g]etty.*tty1' | awk '{print $1}')
if [ -n "$GETTY_PID" ]; then
    kill "$GETTY_PID" 2>/dev/null || true
    sleep 1
fi

chvt 1 2>/dev/null || true

# Launch GUI in background
GUI_LOG="/tmp/controlleros-gui-test.log"
rm -f "$GUI_LOG"
live_msg "Launching controlleros-gui..."
SLINT_KMS_ROTATION=90 RUST_LOG=info "$GUI_BIN" > "$GUI_LOG" 2>&1 &
GUI_PID=$!

sleep 5

# Check 2: GUI process still alive
if kill -0 "$GUI_PID" 2>/dev/null; then
    pass "GUI process still alive after 5 seconds"
else
    fail "GUI process died within 5 seconds"
    echo "--- GUI log ---"
    cat "$GUI_LOG" 2>/dev/null || echo "(no log)"
    echo "--- end GUI log ---"
    echo "=== Results ==="
    echo "PASS: $PASS_COUNT"
    echo "FAIL: $FAIL_COUNT"
    live_msg "test_gui_step6_wired.sh FAILED — GUI crashed"
    exit 1
fi

# Check 3: No panic or fatal in log
if grep -qi 'panic\|fatal' "$GUI_LOG" 2>/dev/null; then
    fail "GUI log contains panic/fatal errors"
else
    pass "GUI log has no panic/fatal errors"
fi

# Check 4: hidd service still running
if pidof hidd >/dev/null 2>&1; then
    pass "hidd service still running"
else
    fail "hidd service NOT running (regression)"
fi

# Manual checks
echo ""
echo "MANUAL: Real Device Population"
echo "MANUAL:   - GUI displays real paired devices (not mock Xbox/DualSense/Pro Controller)"
echo "MANUAL:   - Device list matches the bluetoothctl output above"
echo ""
echo "MANUAL: Disconnect Test"
echo "MANUAL:   - Tap the unlink icon on a connected device"
echo "MANUAL:   - Spinner should appear briefly, status bar shows 'Disconnecting...'"
echo "MANUAL:   - Device row updates to show disconnected state"
echo "MANUAL:   - Host loses controller input from that device"
echo "MANUAL:   - Verify: bluetoothctl info <MAC> shows Connected: no"
echo ""
echo "MANUAL: Reconnect (from host side)"
echo "MANUAL:   - After disconnecting, reconnect from the HOST (not from the Deck GUI)"
echo "MANUAL:   - The GUI should show the device as connected within ~3 seconds"
echo "MANUAL:   - Note: BLE peripherals cannot initiate connections — reconnect is host-initiated"
echo ""
echo "MANUAL: Forget Device Test"
echo "MANUAL:   - First disconnect the device"
echo "MANUAL:   - Tap the delete icon, confirmation dialog appears"
echo "MANUAL:   - Tap 'Confirm' — device disappears from list"
echo "MANUAL:   - Verify: bluetoothctl devices no longer lists the device"
echo "MANUAL:   - WARNING: Device must be re-paired after this test"
echo ""
echo "MANUAL: Auto-Refresh Test"
echo "MANUAL:   - Connect or disconnect a device externally (from host, not GUI)"
echo "MANUAL:   - Wait up to 5 seconds"
echo "MANUAL:   - GUI should update automatically without any button tap"
echo ""
echo "MANUAL: Disconnected Device Display"
echo "MANUAL:   - Disconnected devices should show only the delete icon (no connect button)"
echo "MANUAL:   - Connected devices should show both the unlink icon and delete icon"

live_msg "MANUAL CHECK: Real devices should be displayed. Test disconnect/reconnect/forget/auto-refresh/error handling. Waiting 120 seconds..."

sleep 120

# Diagnostics
echo ""
echo "=== Diagnostics ==="
echo "--- GUI log ---"
cat "$GUI_LOG" 2>/dev/null || echo "(no log)"
echo "--- end GUI log ---"

echo ""
echo "--- bluetoothctl info (all paired) ---"
for mac in $(echo "$BTCTL_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    echo ">> bluetoothctl info $mac"
    bluetoothctl info "$mac" 2>/dev/null || echo "(failed)"
    echo ""
done
echo "--- end bluetoothctl info ---"

# Cleanup
live_msg "Killing GUI process"
kill "$GUI_PID" 2>/dev/null || true
wait "$GUI_PID" 2>/dev/null || true

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step6_wired.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
