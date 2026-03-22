#!/bin/sh
# Test script for GUI Step 4: Full Slint UI layout with std-widgets
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step4_layout.sh

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

live_msg "Starting test_gui_step4_layout.sh"

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

# Kill tty1 getty to free the primary VT for GUI rendering
GETTY_PID=$(ps | grep '[g]etty.*tty1' | awk '{print $1}')
if [ -n "$GETTY_PID" ]; then
    kill "$GETTY_PID" 2>/dev/null || true
    sleep 1
fi

chvt 1 2>/dev/null || true

# Touchscreen diagnostics
echo "=== Touchscreen Diagnostics ==="
for dev in /sys/class/input/event*; do
    NAME=$(cat "$dev/device/name" 2>/dev/null || true)
    if echo "$NAME" | grep -qi "fts\|touch"; then
        EVDEV="/dev/input/$(basename $dev)"
        echo "Touch device: $EVDEV  name: $NAME"
        if command -v udevadm >/dev/null 2>&1; then
            udevadm info "$EVDEV" 2>/dev/null || echo "(udevadm failed)"
        else
            echo "(udevadm not available — eudev tools may not be installed)"
        fi
    fi
done
echo "=== End Touchscreen Diagnostics ==="

# Launch GUI in background with rotation
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
    live_msg "test_gui_step4_layout.sh FAILED — GUI crashed"
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
echo "MANUAL: Verify the full layout is visible:"
echo "MANUAL:   - Header: 'ControllerOS — Bluetooth Devices'"
echo "MANUAL:   - Device list with 3 mock entries (Xbox Controller, DualSense, Pro Controller)"
echo "MANUAL:   - Each row shows: name, MAC address, connection status, Disconnect button (if connected), Forget button"
echo "MANUAL:   - Bottom: Reload, Power Off buttons"
echo "MANUAL:   - Black background, white text"
echo "MANUAL: Tap 'Disconnect' on Xbox Controller — status bar should update"
echo "MANUAL: Tap 'Forget' on any device — confirmation dialog should appear"
echo "MANUAL: Tap 'Cancel' in confirmation dialog — dialog should dismiss"
echo "MANUAL: Tap 'Reload' — confirmation dialog should appear, tap 'Confirm' — status bar updates"
echo "MANUAL: Scroll the device list by touch-dragging"

live_msg "MANUAL CHECK: Full UI layout should be visible. Tap buttons to verify callbacks. Check the GUI log after for tracing output. Waiting 60 seconds..."

sleep 60

# Diagnostics
echo ""
echo "=== Diagnostics ==="
echo "--- GUI log ---"
cat "$GUI_LOG" 2>/dev/null || echo "(no log)"
echo "--- end GUI log ---"

# Cleanup
live_msg "Killing GUI process"
kill "$GUI_PID" 2>/dev/null || true
wait "$GUI_PID" 2>/dev/null || true

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step4_layout.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
