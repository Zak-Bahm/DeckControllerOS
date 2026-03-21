#!/bin/sh
# Test script for GUI Step 5: BlueZ D-Bus client module
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step5_bluez.sh

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

live_msg "Starting test_gui_step5_bluez.sh"

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

# Check 2: --list-devices exits successfully
live_msg "Running --list-devices..."
LIST_OUTPUT=$("$GUI_BIN" --list-devices 2>&1)
LIST_EXIT=$?

echo "=== --list-devices output ==="
echo "$LIST_OUTPUT"
echo "=== end --list-devices output ==="

if [ "$LIST_EXIT" -eq 0 ]; then
    pass "--list-devices exited successfully (code 0)"
else
    fail "--list-devices exited with code $LIST_EXIT"
fi

# Check 3: bluetoothctl paired devices exist
BTCTL_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
BTCTL_EXIT=$?

echo "=== bluetoothctl devices ==="
echo "$BTCTL_DEVICES"
echo "=== end bluetoothctl devices ==="

if [ "$BTCTL_EXIT" -ne 0 ] || [ -z "$BTCTL_DEVICES" ]; then
    echo "WARNING: No paired devices from bluetoothctl — cannot validate device enumeration"
    echo "WARNING: Pair at least one Bluetooth device and re-run this test"
    # Still count as pass for --list-devices if it ran, but skip remaining checks
    echo ""
    echo "=== Results ==="
    echo "PASS: $PASS_COUNT"
    echo "FAIL: $FAIL_COUNT"
    SUMMARY="test_gui_step5_bluez.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT (no paired devices to validate)"
    live_msg "$SUMMARY"
    if [ "$FAIL_COUNT" -gt 0 ]; then
        exit 1
    fi
    exit 0
fi

# Check 4: Every MAC from bluetoothctl appears in --list-devices output
echo ""
echo "=== Validating device enumeration ==="
ALL_MACS_FOUND=true
echo "$BTCTL_DEVICES" | while read -r _device mac _name; do
    if [ -z "$mac" ]; then
        continue
    fi
    if echo "$LIST_OUTPUT" | grep -q "$mac"; then
        echo "  OK: $mac found in --list-devices"
    else
        echo "  MISSING: $mac NOT found in --list-devices"
        ALL_MACS_FOUND=false
    fi
done

# Re-check with a single pass (subshell above can't modify parent vars)
MISSING_MACS=""
echo "$BTCTL_DEVICES" | while read -r _device mac _name; do
    [ -z "$mac" ] && continue
    echo "$LIST_OUTPUT" | grep -q "$mac" || MISSING_MACS="$MISSING_MACS $mac"
done

# Use grep -c to count missing MACs as a workaround for subshell variable scope
MISSING_COUNT=0
for mac in $(echo "$BTCTL_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    if ! echo "$LIST_OUTPUT" | grep -q "$mac"; then
        MISSING_COUNT=$((MISSING_COUNT + 1))
    fi
done

if [ "$MISSING_COUNT" -eq 0 ]; then
    pass "All paired device MACs found in --list-devices output"
else
    fail "$MISSING_COUNT paired device MAC(s) missing from --list-devices output"
fi

# Check 5: Connection status accuracy
echo ""
echo "=== Validating connection status ==="
STATUS_MISMATCHES=0

for mac in $(echo "$BTCTL_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue

    # Get connected status from bluetoothctl info
    BTCTL_INFO=$(bluetoothctl info "$mac" 2>/dev/null)
    BTCTL_CONNECTED=$(echo "$BTCTL_INFO" | grep "Connected:" | awk '{print $2}')

    # Get status from --list-devices (tab-separated: name\taddress\tstatus\tobj_path)
    LIST_LINE=$(echo "$LIST_OUTPUT" | grep "$mac")

    if [ -z "$LIST_LINE" ]; then
        echo "  SKIP: $mac not in --list-devices output"
        continue
    fi

    # Extract the status field (3rd tab-separated field)
    LIST_STATUS=$(echo "$LIST_LINE" | cut -f3)

    if [ "$BTCTL_CONNECTED" = "yes" ]; then
        EXPECTED_STATUS="Connected"
    else
        EXPECTED_STATUS="Disconnected"
    fi

    if [ "$LIST_STATUS" = "$EXPECTED_STATUS" ]; then
        echo "  OK: $mac status=$LIST_STATUS matches bluetoothctl (Connected: $BTCTL_CONNECTED)"
    else
        echo "  MISMATCH: $mac --list-devices=$LIST_STATUS but bluetoothctl Connected=$BTCTL_CONNECTED"
        STATUS_MISMATCHES=$((STATUS_MISMATCHES + 1))
    fi
done

if [ "$STATUS_MISMATCHES" -eq 0 ]; then
    pass "Connection status matches bluetoothctl for all devices"
else
    fail "$STATUS_MISMATCHES device(s) have mismatched connection status"
fi

# Diagnostics
echo ""
echo "=== Diagnostics ==="
echo "--- bluetoothctl info (all paired) ---"
for mac in $(echo "$BTCTL_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    echo ">> bluetoothctl info $mac"
    bluetoothctl info "$mac" 2>/dev/null || echo "(failed)"
    echo ""
done
echo "--- end bluetoothctl info ---"

# Summary
echo ""
echo "=== Results ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

SUMMARY="test_gui_step5_bluez.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
