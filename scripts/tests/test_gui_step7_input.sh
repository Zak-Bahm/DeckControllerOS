#!/bin/sh
# Test script for GUI Step 7: End-to-end input validation
# Verifies that physical Deck controls produce BLE HID reports delivered to a connected host.
#
# Automated checks:
#   - hidraw device discovery and raw report reading
#   - hidraw reports change when physical controls are pressed
#   - hidd is running and publishing BLE reports
#   - GUI is alive alongside hidd without interference
#   - hidd log shows active report publishing (notifying=true)
#
# Manual checks (performed on the host device):
#   - Buttons, sticks, and triggers register as Xbox controller input
#
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step7_input.sh

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

step_banner() {
    echo ""
    echo "========================================"
    echo "  STEP $1: $2"
    echo "========================================"
    live_msg "STEP $1: $2"
}

live_msg "Starting test_gui_step7_input.sh"

# ==========================================================
#  STEP 0: Prerequisites
# ==========================================================
step_banner 0 "Prerequisites"

# Locate binaries
CTL_BIN=""
if [ -x /tmp/dev-update/bin/controllerosctl ]; then
    CTL_BIN="/tmp/dev-update/bin/controllerosctl"
elif [ -x /usr/bin/controllerosctl ]; then
    CTL_BIN="/usr/bin/controllerosctl"
fi
if [ -n "$CTL_BIN" ]; then
    pass "controllerosctl found at $CTL_BIN"
else
    fail "controllerosctl binary not found"
fi

GUI_BIN=""
if [ -x /tmp/dev-update/bin/controlleros-gui ]; then
    GUI_BIN="/tmp/dev-update/bin/controlleros-gui"
elif [ -x /usr/bin/controlleros-gui ]; then
    GUI_BIN="/usr/bin/controlleros-gui"
fi
if [ -n "$GUI_BIN" ]; then
    pass "controlleros-gui found at $GUI_BIN"
else
    fail "controlleros-gui binary not found"
fi

MAPPING_CONFIG=""
if [ -f /etc/controlleros/mapping/xbox.toml ]; then
    MAPPING_CONFIG="/etc/controlleros/mapping/xbox.toml"
elif [ -f /tmp/dev-update/configs/mapping/xbox.toml ]; then
    MAPPING_CONFIG="/tmp/dev-update/configs/mapping/xbox.toml"
fi
if [ -n "$MAPPING_CONFIG" ]; then
    pass "mapping config found at $MAPPING_CONFIG"
else
    fail "mapping config (xbox.toml) not found"
fi

# Verify a host is connected
BTCTL_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
HOST_MAC=""
for mac in $(echo "$BTCTL_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    CONNECTED=$(bluetoothctl info "$mac" 2>/dev/null | grep "Connected:" | awk '{print $2}')
    if [ "$CONNECTED" = "yes" ]; then
        HOST_MAC="$mac"
        break
    fi
done

if [ -n "$HOST_MAC" ]; then
    HOST_NAME=$(bluetoothctl info "$HOST_MAC" 2>/dev/null | grep "Name:" | sed 's/.*Name: //')
    pass "Connected host found: $HOST_NAME ($HOST_MAC)"
else
    fail "No connected host — pair and connect a device before running this test"
    echo ""
    echo "  Paired devices:"
    echo "$BTCTL_DEVICES" | sed 's/^/    /'
    echo ""
    echo "=== Results: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
    live_msg "test_gui_step7_input.sh — no connected host, aborting"
    exit 1
fi

# ==========================================================
#  STEP 1: Hidraw device discovery and raw report reading
# ==========================================================
step_banner 1 "Hidraw raw input validation"

CLIENT_DEV=""
for dev in /sys/class/hidraw/hidraw*; do
    [ -e "$dev" ] || continue
    NAME="$(basename "$dev")"
    UEVENT="$dev/device/uevent"
    [ -f "$UEVENT" ] || continue
    HID_ID="$(grep '^HID_ID=' "$UEVENT" | head -1 | sed 's/HID_ID=//')"
    VID="$(echo "$HID_ID" | cut -d: -f2)"
    case "$VID" in
        *28DE*|*28de*)
            HAS_INPUT="no"
            [ -d "$dev/device/input" ] && HAS_INPUT="yes"
            if [ "$HAS_INPUT" = "no" ]; then
                CLIENT_DEV="/dev/$NAME"
            fi
            ;;
    esac
done

if [ -n "$CLIENT_DEV" ]; then
    pass "Valve hidraw client device: $CLIENT_DEV"
else
    fail "No Valve hidraw client device found"
fi

# Read raw reports for 3 seconds
if [ -n "$CLIENT_DEV" ]; then
    TMP="/tmp/hidraw_input_test"
    rm -f "$TMP"
    dd if="$CLIENT_DEV" of="$TMP" bs=64 2>/dev/null &
    DD_PID=$!
    sleep 3
    kill "$DD_PID" 2>/dev/null
    wait "$DD_PID" 2>/dev/null
    BYTES=0
    [ -f "$TMP" ] && BYTES=$(wc -c < "$TMP")

    if [ "$BYTES" -gt 0 ]; then
        pass "Received $BYTES bytes of raw hidraw data"
        BYTE2="$(od -A n -t x1 -N 1 -j 2 "$TMP" 2>/dev/null | tr -d ' ')"
        if [ "$BYTE2" = "09" ]; then
            pass "Report type 0x09 (DECK_INPUT_REPORT)"
        else
            fail "Unexpected report type 0x$BYTE2 (expected 0x09)"
        fi
    else
        fail "No data received from hidraw device"
    fi
    rm -f "$TMP"
fi

# ==========================================================
#  STEP 2: Physical input changes detected via hidraw
# ==========================================================
step_banner 2 "Physical input change detection"

# hidd holds the hidraw device, so controllerosctl input monitor cannot
# open it simultaneously. Instead, read raw hidraw reports before and
# after user input and compare bytes to detect changes.
if [ -n "$CLIENT_DEV" ]; then
    echo ""
    echo "  Taking a baseline snapshot (keep hands off controls)..."
    SNAP_A="/tmp/hidraw_snap_a"
    SNAP_B="/tmp/hidraw_snap_b"
    rm -f "$SNAP_A" "$SNAP_B"

    # Capture one 64-byte report as baseline
    dd if="$CLIENT_DEV" of="$SNAP_A" bs=64 count=1 2>/dev/null

    echo ""
    echo "  >>> PRESS A BUTTON OR MOVE A STICK NOW <<<"
    live_msg "PRESS A BUTTON OR MOVE A STICK on the Deck"
    sleep 5

    # Capture another report after user input
    dd if="$CLIENT_DEV" of="$SNAP_B" bs=64 count=1 2>/dev/null

    if [ -f "$SNAP_A" ] && [ -f "$SNAP_B" ]; then
        HEX_A=$(od -A n -t x1 "$SNAP_A" 2>/dev/null | tr -d ' \n')
        HEX_B=$(od -A n -t x1 "$SNAP_B" 2>/dev/null | tr -d ' \n')

        if [ -n "$HEX_A" ] && [ -n "$HEX_B" ]; then
            pass "Captured before/after hidraw snapshots"
            if [ "$HEX_A" != "$HEX_B" ]; then
                pass "Hidraw reports differ — physical input change detected"
            else
                fail "Hidraw reports identical — did you press a button or move a stick?"
            fi
        else
            fail "Could not read hidraw snapshots"
        fi
    else
        fail "Hidraw snapshot files missing"
    fi
    rm -f "$SNAP_A" "$SNAP_B"
else
    echo "  SKIPPED — no hidraw client device found in step 1"
fi

# ==========================================================
#  STEP 3: hidd is running and publishing BLE reports
# ==========================================================
step_banner 3 "hidd BLE report publishing"

if pidof hidd >/dev/null 2>&1; then
    pass "hidd process is running"
else
    fail "hidd process is NOT running"
    echo ""
    echo "  Attempting to start hidd..."
    if [ -x /etc/init.d/S45hidd ]; then
        /etc/init.d/S45hidd start >/dev/null 2>&1 || true
        sleep 3
        if pidof hidd >/dev/null 2>&1; then
            pass "hidd started successfully"
        else
            fail "hidd failed to start"
        fi
    fi
fi

# Check hidd log for evidence of active BLE notification delivery
HIDD_LOG="/var/log/hidd.log"
if [ -f "$HIDD_LOG" ]; then
    # Look for notification enable (either StartNotify from host or restored on reconnect)
    if grep -q "StartNotify input_report" "$HIDD_LOG" 2>/dev/null; then
        pass "hidd log shows host subscribed to input reports (StartNotify)"
    elif grep -q "restored notifying flags on connect" "$HIDD_LOG" 2>/dev/null; then
        pass "hidd log shows notifying flags restored on reconnect"
    else
        fail "hidd log has no StartNotify or restored flags — reports may not be sent"
    fi

    # Look for connection events
    if grep -q "device connected:" "$HIDD_LOG" 2>/dev/null; then
        pass "hidd log shows device connection event"
    else
        fail "hidd log has no connection event"
    fi

    # Check for errors that would block report delivery
    if grep -q "failed to emit input report notification" "$HIDD_LOG" 2>/dev/null; then
        fail "hidd log contains report notification errors"
    else
        pass "hidd log has no report notification errors"
    fi
else
    fail "hidd log not found at $HIDD_LOG"
fi

# ==========================================================
#  STEP 4: GUI runs alongside hidd without interference
# ==========================================================
step_banner 4 "GUI + hidd coexistence"

if [ -n "$GUI_BIN" ]; then
    # Kill tty1 getty to free the primary VT
    GETTY_PID=$(ps | grep '[g]etty.*tty1' | awk '{print $1}')
    if [ -n "$GETTY_PID" ]; then
        kill "$GETTY_PID" 2>/dev/null || true
        sleep 1
    fi
    chvt 1 2>/dev/null || true

    GUI_LOG="/tmp/controlleros-gui-input-test.log"
    rm -f "$GUI_LOG"
    SLINT_KMS_ROTATION=90 RUST_LOG=info "$GUI_BIN" > "$GUI_LOG" 2>&1 &
    GUI_PID=$!

    sleep 5

    if kill -0 "$GUI_PID" 2>/dev/null; then
        pass "GUI process alive after 5 seconds"
    else
        fail "GUI process died"
        echo "--- GUI log ---"
        cat "$GUI_LOG" 2>/dev/null
        echo "--- end ---"
    fi

    if pidof hidd >/dev/null 2>&1; then
        pass "hidd still running alongside GUI"
    else
        fail "hidd died after GUI launch"
    fi

    if grep -qi 'panic\|fatal' "$GUI_LOG" 2>/dev/null; then
        fail "GUI log contains panic/fatal errors"
    else
        pass "GUI log clean (no panic/fatal)"
    fi

    # Verify host is still connected after GUI launch
    STILL_CONNECTED=$(bluetoothctl info "$HOST_MAC" 2>/dev/null | grep "Connected:" | awk '{print $2}')
    if [ "$STILL_CONNECTED" = "yes" ]; then
        pass "Host $HOST_MAC still connected after GUI launch"
    else
        fail "Host $HOST_MAC disconnected after GUI launch"
    fi
else
    echo "  SKIPPED — controlleros-gui not found"
fi

# ==========================================================
#  STEP 5: End-to-end input with GUI running (manual)
# ==========================================================
step_banner 5 "End-to-end input verification (manual)"

echo ""
echo "  The GUI is now running alongside hidd."
echo "  On your HOST DEVICE ($HOST_NAME), open a gamepad tester"
echo "  (e.g. gamepad-tester.com in a browser, or Steam controller settings)."
echo ""
echo "  MANUAL CHECKS:"
echo "    1. Press A/B/X/Y on the Deck — buttons register on the host"
echo "    2. Press LB/RB — shoulder buttons register"
echo "    3. Move left stick — left analog registers on the host"
echo "    4. Move right stick — right analog registers on the host"
echo "    5. Press left/right triggers — analog triggers register"
echo "    6. Press d-pad directions — hat/POV registers"
echo "    7. Press Start/Select/Home — remaining buttons register"
echo ""
echo "  If no input appears on the host:"
echo "    - Check that the host shows the device as an Xbox controller"
echo "    - Verify hidd log shows StartNotify (the host subscribed)"
echo "    - Verify no 'reset all notifying flags' AFTER the connection"
echo ""

live_msg "MANUAL CHECK: On the host ($HOST_NAME), verify Deck buttons/sticks/triggers register as Xbox controller input. Waiting 60 seconds..."

sleep 60

# ==========================================================
#  Diagnostics
# ==========================================================
echo ""
echo "========================================"
echo "  DIAGNOSTICS"
echo "========================================"

echo ""
echo "--- hidd log (last 50 lines) ---"
if [ -f "$HIDD_LOG" ]; then
    tail -n 50 "$HIDD_LOG"
else
    echo "(not found)"
fi
echo "--- end hidd log ---"

if [ -n "$GUI_LOG" ] && [ -f "$GUI_LOG" ]; then
    echo ""
    echo "--- GUI log (last 30 lines) ---"
    tail -n 30 "$GUI_LOG"
    echo "--- end GUI log ---"
fi

echo ""
echo "--- bluetoothctl info $HOST_MAC ---"
bluetoothctl info "$HOST_MAC" 2>/dev/null || echo "(failed)"
echo "--- end ---"

# ==========================================================
#  Cleanup
# ==========================================================
if [ -n "${GUI_PID:-}" ]; then
    kill "$GUI_PID" 2>/dev/null || true
    wait "$GUI_PID" 2>/dev/null || true
fi

# ==========================================================
#  Summary
# ==========================================================
echo ""
echo "========================================"
echo "  RESULTS"
echo "========================================"
echo "  PASS: $PASS_COUNT"
echo "  FAIL: $FAIL_COUNT"
echo "========================================"

SUMMARY="test_gui_step7_input.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
