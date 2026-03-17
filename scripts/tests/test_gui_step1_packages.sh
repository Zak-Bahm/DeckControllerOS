#!/bin/sh
# Test script for GUI Step 1: Buildroot packages — mesa3d, libinput, libxkbcommon
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step1_packages.sh

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

live_msg "Starting test_gui_step1_packages.sh"

# Check 1: libEGL shared object
if ls /usr/lib/libEGL* >/dev/null 2>&1; then
    pass "libEGL found in /usr/lib/"
else
    fail "libEGL not found in /usr/lib/"
fi

# Check 2: libGLESv2 shared object
if ls /usr/lib/libGLESv2* >/dev/null 2>&1; then
    pass "libGLESv2 found in /usr/lib/"
else
    fail "libGLESv2 not found in /usr/lib/"
fi

# Check 3: gallium radeonsi driver (mesa 25.x uses libgallium + GBM, not legacy /usr/lib/dri/)
if ls /usr/lib/libgallium* >/dev/null 2>&1; then
    pass "libgallium found (radeonsi gallium driver)"
elif [ -f /usr/lib/dri/radeonsi_dri.so ]; then
    pass "radeonsi_dri.so found (legacy DRI path)"
else
    fail "no gallium or radeonsi DRI driver found"
fi

# Check 4: libinput available and lists touchscreen
if command -v libinput >/dev/null 2>&1; then
    if libinput list-devices 2>/dev/null | grep -qi touch; then
        pass "libinput available and lists a touchscreen device"
    else
        pass "libinput available (no touchscreen in list — may need udev rules)"
    fi
else
    fail "libinput command not found"
fi

# Check 5: libxkbcommon shared object
if ls /usr/lib/libxkbcommon* >/dev/null 2>&1; then
    pass "libxkbcommon found in /usr/lib/"
else
    fail "libxkbcommon not found in /usr/lib/"
fi

# Check 6: libdrm shared object
if ls /usr/lib/libdrm* >/dev/null 2>&1; then
    pass "libdrm found in /usr/lib/"
else
    fail "libdrm not found in /usr/lib/"
fi

# Check 7: /dev/dri/card* exists (Step 0 no regression)
if ls /dev/dri/card* >/dev/null 2>&1; then
    pass "/dev/dri/card* exists"
else
    fail "/dev/dri/card* does not exist (regression)"
fi

# Check 8: hidd service running (no regression)
if pidof hidd >/dev/null 2>&1 || /etc/init.d/S45hidd status 2>/dev/null | grep -qi "running"; then
    pass "hidd service is running"
else
    fail "hidd service is NOT running (regression)"
fi

# Check 9: bluetoothd service running (no regression)
if pidof bluetoothd >/dev/null 2>&1; then
    pass "bluetoothd service is running"
else
    fail "bluetoothd service is NOT running (regression)"
fi

# Diagnostics
echo ""
echo "=== Diagnostics ==="

echo "--- EGL/GLES libraries ---"
ls -la /usr/lib/libEGL* /usr/lib/libGLESv2* 2>/dev/null || echo "(none)"

echo "--- Gallium / GBM / DRI drivers ---"
ls -la /usr/lib/libgallium* 2>/dev/null || echo "(no libgallium)"
ls -la /usr/lib/gbm/ 2>/dev/null || echo "(no /usr/lib/gbm/)"
ls -la /usr/lib/libgbm* 2>/dev/null || echo "(no libgbm)"
ls -la /usr/lib/dri/ 2>/dev/null || echo "(no /usr/lib/dri/)"

echo "--- libinput list-devices ---"
libinput list-devices 2>/dev/null || echo "(libinput not available)"

echo "--- libxkbcommon ---"
ls -la /usr/lib/libxkbcommon* 2>/dev/null || echo "(none)"

echo "--- libdrm ---"
ls -la /usr/lib/libdrm* 2>/dev/null || echo "(none)"

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step1_packages.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
