#!/bin/sh
# Test script for GUI Step 8: Init script, auto-start, and startup splash screen
# Run AFTER a fresh boot — do NOT launch GUI manually first.
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step8_autostart.sh

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

live_msg "Starting test_gui_step8_autostart.sh"

# Check 1: GUI process already running (auto-started by S01gui)
if pidof controlleros-gui >/dev/null 2>&1; then
    pass "controlleros-gui is already running (auto-started)"
else
    fail "controlleros-gui is NOT running — S01gui may not have started it"
fi

# Check 2: S01gui init script exists and is executable
if [ -x /etc/init.d/S01gui ]; then
    pass "/etc/init.d/S01gui exists and is executable"
else
    fail "/etc/init.d/S01gui not found or not executable"
fi

# Check 3: GUI log exists and has content
GUI_LOG="/var/log/controlleros-gui.log"
if [ -f "$GUI_LOG" ] && [ -s "$GUI_LOG" ]; then
    pass "GUI log exists and has content at $GUI_LOG"
else
    fail "GUI log missing or empty at $GUI_LOG"
fi

# Check 4: No panic/fatal in GUI log
if [ -f "$GUI_LOG" ]; then
    if grep -qi 'panic\|fatal' "$GUI_LOG" 2>/dev/null; then
        fail "GUI log contains panic/fatal errors"
    else
        pass "GUI log has no panic/fatal errors"
    fi
fi

# Check 5: hidd and bluetoothd running
if pidof hidd >/dev/null 2>&1; then
    pass "hidd service is running"
else
    fail "hidd service is NOT running"
fi

if pidof bluetoothd >/dev/null 2>&1; then
    pass "bluetoothd service is running"
else
    fail "bluetoothd service is NOT running"
fi

# Check 6: S01gui stop/start/restart cycle
echo ""
echo "=== Init script stop/start/restart test ==="

live_msg "Testing S01gui stop..."
/etc/init.d/S01gui stop >/dev/null 2>&1
sleep 2
if ! pidof controlleros-gui >/dev/null 2>&1; then
    pass "S01gui stop — GUI process stopped"
else
    fail "S01gui stop — GUI process still running"
fi

live_msg "Testing S01gui start..."
/etc/init.d/S01gui start >/dev/null 2>&1
sleep 5
if pidof controlleros-gui >/dev/null 2>&1; then
    pass "S01gui start — GUI process running"
else
    fail "S01gui start — GUI process not running"
fi

live_msg "Testing S01gui restart..."
/etc/init.d/S01gui restart >/dev/null 2>&1
sleep 5
if pidof controlleros-gui >/dev/null 2>&1; then
    pass "S01gui restart — GUI process running"
else
    fail "S01gui restart — GUI process not running"
fi

# Check 7: tty2 and tty3 gettys running (debug terminals)
if ps | grep -q '[g]etty.*tty2'; then
    pass "tty2 getty is running (debug terminal available)"
else
    fail "tty2 getty is NOT running"
fi

if ps | grep -q '[g]etty.*tty3'; then
    pass "tty3 getty is running (debug terminal available)"
else
    fail "tty3 getty is NOT running"
fi

# Check 8: tty1 getty NOT running (freed for GUI)
if ps | grep -q '[g]etty.*tty1'; then
    fail "tty1 getty is still running — should be disabled for GUI"
else
    pass "tty1 getty is not running (freed for GUI)"
fi

# Manual checks
echo ""
echo "MANUAL: Splash screen and auto-start tests:"
echo ""
echo "MANUAL: On boot (before this test script):"
echo "MANUAL:   1. No kernel/init text visible on screen (quiet boot)"
echo "MANUAL:   2. Splash screen appeared first: 'ControllerOS' centered with spinning indicator"
echo "MANUAL:   3. After a few seconds, spinner stopped and splash faded out smoothly"
echo "MANUAL:   4. Main UI (device list, system buttons) now visible"
echo ""
echo "MANUAL: Display and touch:"
echo "MANUAL:   - GUI is landscape (1280x800)"
echo "MANUAL:   - Touch works correctly on all buttons"
echo "MANUAL:   - Tap buttons in each corner to verify touch alignment"
echo ""
echo "MANUAL: VT switching:"
echo "MANUAL:   - chvt 2 -> login prompt visible on tty2"
echo "MANUAL:   - chvt 1 -> GUI still visible"
echo ""
echo "MANUAL: Controller:"
echo "MANUAL:   - Pair a host -> controller input works while GUI is displayed"
echo ""

live_msg "MANUAL CHECK: Verify quiet boot, splash appeared, faded to main UI. Test touch, VT switching. Waiting 60 seconds..."

sleep 60

# Diagnostics
echo ""
echo "=== Diagnostics ==="
echo "--- GUI log (last 30 lines) ---"
if [ -f "$GUI_LOG" ]; then
    tail -n 30 "$GUI_LOG"
else
    echo "(not found)"
fi
echo "--- end GUI log ---"

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step8_autostart.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
