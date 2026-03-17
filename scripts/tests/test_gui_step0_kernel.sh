#!/bin/sh
# Test script for GUI Step 0: Kernel config — AMD GPU and touchscreen drivers
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step0_kernel.sh

PASS_COUNT=0
FAIL_COUNT=0

# Send a live message to the dev-payload server via POST /send-instruction so
# the tester can follow along in real time on the server console.
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

pass() {
    echo "PASS: $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
    echo "FAIL: $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

live_msg "Starting test_gui_step0_kernel.sh — automated checks running..."

# Check 1: /dev/dri/card* exists
if ls /dev/dri/card* >/dev/null 2>&1; then
    pass "/dev/dri/card* exists"
    echo "  DRI devices:"
    ls -l /dev/dri/card* 2>/dev/null
else
    fail "/dev/dri/card* does not exist"
fi

# Check 2: amdgpu driver initialization in dmesg
if dmesg | grep -qi "amdgpu"; then
    pass "dmesg contains amdgpu driver messages"
    echo "  amdgpu dmesg lines (first 10):"
    dmesg | grep -i "amdgpu" | head -10
else
    fail "dmesg does not contain amdgpu driver messages"
fi

# Check 3: Touchscreen input device found in /proc/bus/input/devices
TOUCH_FOUND=0
if [ -f /proc/bus/input/devices ]; then
    # Look for touchscreen or multitouch devices
    if grep -qi -E "touch|multitouch|FTS" /proc/bus/input/devices; then
        pass "Touchscreen input device found in /proc/bus/input/devices"
        TOUCH_FOUND=1
    else
        fail "No touchscreen input device found in /proc/bus/input/devices"
    fi
else
    fail "/proc/bus/input/devices does not exist"
fi

# Check 4: hidd service is running (no regression)
if pidof hidd >/dev/null 2>&1 || /etc/init.d/S45hidd status 2>/dev/null | grep -qi "running"; then
    pass "hidd service is running"
else
    fail "hidd service is NOT running (regression)"
fi

# Check 5: bluetoothd service is running (no regression)
if pidof bluetoothd >/dev/null 2>&1; then
    pass "bluetoothd service is running"
else
    fail "bluetoothd service is NOT running (regression)"
fi

# Diagnostics dump
echo ""
echo "=== Diagnostics ==="

echo "--- DRI device listing ---"
ls -la /dev/dri/ 2>/dev/null || echo "(no /dev/dri/)"

echo "--- /proc/bus/input/devices ---"
cat /proc/bus/input/devices 2>/dev/null || echo "(not available)"

echo "--- amdgpu dmesg (full) ---"
dmesg | grep -i "amdgpu" 2>/dev/null || echo "(none)"

echo "--- DRM dmesg ---"
dmesg | grep -i "drm" | head -20

echo "--- i2c dmesg ---"
dmesg | grep -i "i2c" | head -20

echo "--- touchscreen/multitouch dmesg ---"
dmesg | grep -i -E "touch|multitouch|hid-multitouch" | head -20

# Manual checks — sent live so the tester can act on them during the run
echo ""
if [ "$TOUCH_FOUND" = "1" ]; then
    TOUCH_EVENT=$(grep -B5 -i -E "touch|multitouch|FTS" /proc/bus/input/devices 2>/dev/null | grep "Handlers=" | grep -o "event[0-9]*" | head -1)
    if [ -n "$TOUCH_EVENT" ]; then
        MANUAL_MSG="MANUAL: Tap the touchscreen while running: cat /dev/input/$TOUCH_EVENT — binary data should appear"
    else
        MANUAL_MSG="MANUAL: Tap the touchscreen while running: cat /dev/input/<touchscreen_event> — binary data should appear"
    fi
else
    MANUAL_MSG="MANUAL: Touchscreen device not auto-detected. Check dmesg for touchscreen driver info and test manually."
fi
echo "$MANUAL_MSG"
live_msg "$MANUAL_MSG"

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step0_kernel.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
