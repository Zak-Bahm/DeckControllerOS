#!/bin/sh
# Test script for GUI Step 3: First render on Deck via DRM/KMS
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step3_render.sh

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

live_msg "Starting test_gui_step3_render.sh"

# Check 1: Locate controlleros-gui binary
GUI_BIN=""
if [ -x /usr/bin/controlleros-gui ]; then
    GUI_BIN="/usr/bin/controlleros-gui"
    pass "controlleros-gui found at /usr/bin/controlleros-gui"
elif [ -x /tmp/dev-update/bin/controlleros-gui ]; then
    GUI_BIN="/tmp/dev-update/bin/controlleros-gui"
    pass "controlleros-gui found at /tmp/dev-update/bin/controlleros-gui"
else
    fail "controlleros-gui binary not found"
    echo "=== Results ==="
    echo "PASS: $PASS_COUNT"
    echo "FAIL: $FAIL_COUNT"
    exit 1
fi

# Kill tty1 getty to free the primary VT for GUI rendering
live_msg "Killing tty1 getty to free primary VT"
GETTY_PID=$(ps | grep '[g]etty.*tty1' | awk '{print $1}')
if [ -n "$GETTY_PID" ]; then
    kill "$GETTY_PID" 2>/dev/null || true
    sleep 1
    echo "Killed tty1 getty (pid $GETTY_PID)"
else
    echo "No tty1 getty found (already free)"
fi

# Switch to VT1 so the GUI renders on the visible console
chvt 1 2>/dev/null || true

# Launch GUI in background with 90° clockwise rotation for Steam Deck portrait display
GUI_LOG="/tmp/controlleros-gui-test.log"
rm -f "$GUI_LOG"
live_msg "Launching controlleros-gui..."
SLINT_KMS_ROTATION=90 RUST_LOG=info "$GUI_BIN" > "$GUI_LOG" 2>&1 &
GUI_PID=$!
echo "Launched controlleros-gui (pid $GUI_PID)"

# Wait for startup
sleep 5

# Check 2: GUI process still alive
if kill -0 "$GUI_PID" 2>/dev/null; then
    pass "GUI process still alive after 5 seconds"
else
    fail "GUI process died within 5 seconds"
    echo "--- GUI log ---"
    cat "$GUI_LOG" 2>/dev/null || echo "(no log)"
    echo "--- end GUI log ---"
    echo ""
    echo "=== Results ==="
    echo "PASS: $PASS_COUNT"
    echo "FAIL: $FAIL_COUNT"
    live_msg "test_gui_step3_render.sh FAILED — GUI crashed on startup"
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

# Check 5: bluetoothd service still running
if pidof bluetoothd >/dev/null 2>&1; then
    pass "bluetoothd service still running"
else
    fail "bluetoothd service NOT running (regression)"
fi

# Manual checks
echo ""
echo "MANUAL: Verify the Deck screen shows the Slint placeholder window (title 'ControllerOS' + 'Hello from Slint' button)"
echo "MANUAL: Verify the display is landscape (1280x800). If portrait, SLINT_KMS_ROTATION=270 is needed."
echo "MANUAL: Tap the 'Hello from Slint' button — verify visual press feedback."
echo "MANUAL: Tap in different screen areas — verify touch coordinates align with displayed elements."

live_msg "MANUAL CHECK: Look at the Deck screen — you should see the Slint GUI with 'ControllerOS' title and a button. Tap the button to verify touch works. Waiting 30 seconds..."

# Give time for manual inspection
sleep 30

# Diagnostics
echo ""
echo "=== Diagnostics ==="
echo "--- GUI log ---"
cat "$GUI_LOG" 2>/dev/null || echo "(no log)"
echo "--- end GUI log ---"

echo "--- DRI devices ---"
ls -la /dev/dri/ 2>/dev/null || echo "(none)"

echo "--- EGL info ---"
ls -la /usr/lib/libEGL* 2>/dev/null || echo "(none)"

# Cleanup: kill GUI
live_msg "Killing GUI process"
kill "$GUI_PID" 2>/dev/null || true
wait "$GUI_PID" 2>/dev/null || true

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step3_render.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
