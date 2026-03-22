#!/bin/sh
# Test script for GUI Step 7: System actions (reload stack, power off)
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step7_system.sh

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

live_msg "Starting test_gui_step7_system.sh"

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

# Check 2: Init scripts exist and are executable
if [ -x /etc/init.d/S45hidd ]; then
    pass "S45hidd init script exists and is executable"
else
    fail "S45hidd init script not found or not executable"
fi

if [ -x /etc/init.d/S40bluetoothd ]; then
    pass "S40bluetoothd init script exists and is executable"
else
    fail "S40bluetoothd init script not found or not executable"
fi

# Check 3: System commands available
if [ -x /sbin/poweroff ]; then
    pass "/sbin/poweroff is available"
else
    fail "/sbin/poweroff not found"
fi

# Check 4: Snapshot current service state
echo ""
echo "=== Service State Before GUI ==="
if pidof hidd >/dev/null 2>&1; then
    pass "hidd is running"
    echo "hidd PID: $(pidof hidd)"
else
    fail "hidd is NOT running"
fi

if pidof bluetoothd >/dev/null 2>&1; then
    pass "bluetoothd is running"
    echo "bluetoothd PID: $(pidof bluetoothd)"
else
    fail "bluetoothd is NOT running"
fi

# Kill tty1 getty to free the primary VT for GUI rendering
GETTY_PID=$(ps | grep '[g]etty.*tty1' | awk '{print $1}')
if [ -n "$GETTY_PID" ]; then
    kill "$GETTY_PID" 2>/dev/null || true
    sleep 1
fi

chvt 1 2>/dev/null || true

# Launch GUI in background
GUI_LOG="/tmp/controlleros-gui-system-test.log"
rm -f "$GUI_LOG"
live_msg "Launching controlleros-gui..."
SLINT_KMS_ROTATION=90 RUST_LOG=info "$GUI_BIN" > "$GUI_LOG" 2>&1 &
GUI_PID=$!

sleep 5

# Check 5: GUI process still alive
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
    live_msg "test_gui_step7_system.sh FAILED — GUI crashed"
    exit 1
fi

# Check 6: No panic or fatal in log
if grep -qi 'panic\|fatal' "$GUI_LOG" 2>/dev/null; then
    fail "GUI log contains panic/fatal errors"
else
    pass "GUI log has no panic/fatal errors"
fi

# Manual checks
echo ""
echo "MANUAL: System action tests:"
echo ""
echo "MANUAL: Cancel test:"
echo "MANUAL:   1. Tap 'Reload' — confirmation dialog appears"
echo "MANUAL:   2. Tap 'Cancel' — dialog dismisses, nothing happens"
echo "MANUAL:   3. Tap 'Power Off' — confirmation dialog appears"
echo "MANUAL:   4. Tap 'Cancel' — dialog dismisses, nothing happens"
echo ""
echo "MANUAL: Battery indicator:"
echo "MANUAL:   - Battery percentage and progress bar visible above system buttons"
echo ""
echo "MANUAL: Reload Stack test:"
echo "MANUAL:   1. Tap 'Reload' — confirmation dialog appears"
echo "MANUAL:   2. Tap 'Confirm' — status shows 'Reloading stack...'"
echo "MANUAL:   3. GUI should disappear briefly as services cycle and GUI re-execs"
echo "MANUAL:   4. GUI should reappear automatically"
echo "MANUAL:   5. From tty2 (chvt 2): verify 'pidof hidd' and 'pidof bluetoothd' return PIDs"
echo "MANUAL:   6. Reconnect host if needed — controller input still works"
echo "MANUAL:   7. Device list should repopulate in the GUI"
echo ""
echo "MANUAL: Power Off test (DO THIS LAST):"
echo "MANUAL:   1. Tap 'Power Off' — confirmation dialog appears"
echo "MANUAL:   2. Tap 'Confirm' — Deck should shut down cleanly"
echo ""

live_msg "MANUAL CHECK: Test Cancel on Reload and Power Off dialogs. Then test Reload (stack cycles, GUI re-execs). Finally test Power Off (Deck shuts down). Waiting 120 seconds..."

sleep 120

# Diagnostics
echo ""
echo "=== Diagnostics ==="
echo "--- GUI log ---"
cat "$GUI_LOG" 2>/dev/null || echo "(no log)"
echo "--- end GUI log ---"

echo ""
echo "=== Post-test Service State ==="
echo "hidd running: $(pidof hidd 2>/dev/null && echo yes || echo no)"
echo "bluetoothd running: $(pidof bluetoothd 2>/dev/null && echo yes || echo no)"

# Cleanup
live_msg "Killing GUI process"
kill "$GUI_PID" 2>/dev/null || true
wait "$GUI_PID" 2>/dev/null || true

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step7_system.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
