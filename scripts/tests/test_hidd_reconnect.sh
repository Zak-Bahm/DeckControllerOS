#!/bin/sh
# Test script for hidd reconnection: auto-trust and advertisement re-registration
# Guided interactive test — gives directions and waits for each step.
# Run on Deck via:
#   controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_hidd_reconnect.sh

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

countdown() {
    SECS=$1
    LABEL=${2:-"Waiting"}
    while [ "$SECS" -gt 0 ]; do
        printf "\r  %s... %2ds remaining  " "$LABEL" "$SECS"
        sleep 1
        SECS=$((SECS - 1))
    done
    printf "\r  %s... done.                \n" "$LABEL"
}

device_info() {
    MAC=$1
    bluetoothctl info "$MAC" 2>/dev/null
}

get_field() {
    # get_field <mac> <FieldName> -> value (e.g. get_field AA:BB:CC Trusted -> yes)
    device_info "$1" | grep "$2:" | awk '{print $2}'
}

dump_device() {
    MAC=$1
    echo "  --- bluetoothctl info $MAC ---"
    device_info "$MAC" | grep -E "Name:|Paired:|Trusted:|Connected:|Alias:" | sed 's/^/    /'
    echo "  ---"
}

live_msg "Starting test_hidd_reconnect.sh"

# ---- Prerequisites ----
step_banner 0 "Prerequisites"

HIDD_BIN=""
if [ -x /usr/bin/hidd ]; then
    HIDD_BIN="/usr/bin/hidd"
    pass "hidd found at $HIDD_BIN"
elif [ -x /usr/bin/controlleros-hidd ]; then
    HIDD_BIN="/usr/bin/controlleros-hidd"
    pass "hidd found at $HIDD_BIN"
else
    fail "hidd binary not found"
fi

if ! command -v bluetoothctl >/dev/null 2>&1; then
    fail "bluetoothctl not found — cannot continue"
    echo "=== Results: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
    exit 1
fi
pass "bluetoothctl available"

# Restart bluetooth and hidd to ensure we're testing the latest binary
# and that the HCI controller has clean advertising state.
echo ""
echo "  Stopping hidd..."
if [ -x /etc/init.d/S45hidd ]; then
    /etc/init.d/S45hidd stop >/dev/null 2>&1 || true
fi
# Truncate hidd log so diagnostics only show this run.
: > /var/log/hidd.log 2>/dev/null || true
# Power-cycle the adapter while bluetoothd is still running to clear
# stale HCI advertising state ("Unexpected advertising set terminated").
echo "  Power-cycling Bluetooth adapter..."
bluetoothctl power off >/dev/null 2>&1 || true
sleep 1
echo "  Stopping bluetoothd..."
if [ -x /etc/init.d/S40bluetoothd ]; then
    /etc/init.d/S40bluetoothd stop >/dev/null 2>&1 || true
    sleep 1
fi
echo "  Starting bluetoothd..."
if [ -x /etc/init.d/S40bluetoothd ]; then
    /etc/init.d/S40bluetoothd start >/dev/null 2>&1 || true
    sleep 2
fi
# S41bluetooth-power runs bluetoothctl power on; re-run after restart.
if [ -x /etc/init.d/S41bluetooth-power ]; then
    /etc/init.d/S41bluetooth-power >/dev/null 2>&1 || true
fi
echo "  Starting hidd..."
if [ -x /etc/init.d/S45hidd ]; then
    /etc/init.d/S45hidd start >/dev/null 2>&1 || true
    sleep 3
fi

if pidof hidd >/dev/null 2>&1; then
    pass "hidd is running after restart"
elif pidof controlleros-hidd >/dev/null 2>&1; then
    pass "controlleros-hidd is running after restart"
else
    fail "hidd is NOT running after restart"
fi

if pidof bluetoothd >/dev/null 2>&1; then
    pass "bluetoothd is running"
else
    fail "bluetoothd is NOT running"
fi

# Remove any previously paired devices so we start clean
echo ""
echo "  Removing all previously paired devices for a clean test..."
OLD_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
for mac in $(echo "$OLD_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    echo "  Removing $mac..."
    bluetoothctl remove "$mac" >/dev/null 2>&1 || true
done
echo "  Done — starting with a clean slate."

# ==========================================================
#  STEP 1: Pair a new host
# ==========================================================
step_banner 1 "Pair and connect a new host"
echo ""
echo "  ACTION REQUIRED:"
echo "    On your host device (phone/PC/tablet), open Bluetooth settings"
echo "    and pair with this controller."
echo ""
echo "    The controller is advertising and waiting for a connection."
echo ""

countdown 30 "Waiting for host to pair"

# Check for a newly paired device
PAIRED_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
HOST_MAC=""
for mac in $(echo "$PAIRED_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    HOST_MAC="$mac"
    break
done

if [ -z "$HOST_MAC" ]; then
    fail "No paired device found after 30s — did you pair from the host?"
    echo ""
    echo "  Giving extra time..."
    countdown 30 "Extended wait for pairing"

    PAIRED_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
    for mac in $(echo "$PAIRED_DEVICES" | awk '{print $2}'); do
        [ -z "$mac" ] && continue
        HOST_MAC="$mac"
        break
    done
fi

if [ -z "$HOST_MAC" ]; then
    fail "No paired device found — cannot continue"
    HIDD_LOG="/var/log/hidd.log"
    echo ""
    echo "--- hidd log (last 100 lines from $HIDD_LOG) ---"
    if [ -f "$HIDD_LOG" ]; then
        tail -n 100 "$HIDD_LOG"
    else
        echo "(hidd log not found at $HIDD_LOG)"
    fi
    echo "--- end hidd log ---"
    echo "=== Results: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
    live_msg "test_hidd_reconnect.sh — no device paired, aborting"
    exit 1
fi

echo ""
echo "  Found paired host: $HOST_MAC"
dump_device "$HOST_MAC"
pass "Host $HOST_MAC paired successfully"

# Verify Trusted was set automatically
TRUSTED=$(get_field "$HOST_MAC" "Trusted")
if [ "$TRUSTED" = "yes" ]; then
    pass "Host $HOST_MAC is auto-Trusted"
else
    fail "Host $HOST_MAC is NOT Trusted (auto-trust failed)"
fi

# Verify Connected
CONNECTED=$(get_field "$HOST_MAC" "Connected")
if [ "$CONNECTED" = "yes" ]; then
    pass "Host $HOST_MAC is connected"
else
    fail "Host $HOST_MAC is not connected after pairing"
fi

# ==========================================================
#  STEP 2: Host disconnects
# ==========================================================
step_banner 2 "Disconnect from the host side"
echo ""
echo "  ACTION REQUIRED:"
echo "    On your host device, DISCONNECT (do not unpair/forget)"
echo "    the controller from Bluetooth settings."
echo ""

countdown 15 "Waiting for host to disconnect"

CONNECTED=$(get_field "$HOST_MAC" "Connected")
if [ "$CONNECTED" != "yes" ]; then
    pass "Host $HOST_MAC disconnected"
else
    echo "  Device still shows connected — waiting a bit longer..."
    countdown 10 "Extended wait for disconnect"
    CONNECTED=$(get_field "$HOST_MAC" "Connected")
    if [ "$CONNECTED" != "yes" ]; then
        pass "Host $HOST_MAC disconnected"
    else
        fail "Host $HOST_MAC still connected — did you disconnect from the host?"
    fi
fi

dump_device "$HOST_MAC"

# Verify still paired and trusted after disconnect
PAIRED=$(get_field "$HOST_MAC" "Paired")
TRUSTED=$(get_field "$HOST_MAC" "Trusted")
if [ "$PAIRED" = "yes" ]; then
    pass "Host $HOST_MAC still Paired after disconnect"
else
    fail "Host $HOST_MAC lost Paired status after disconnect"
fi
if [ "$TRUSTED" = "yes" ]; then
    pass "Host $HOST_MAC still Trusted after disconnect"
else
    fail "Host $HOST_MAC lost Trusted status after disconnect"
fi

# ==========================================================
#  STEP 3: Host reconnects
# ==========================================================
step_banner 3 "Reconnect from the host side"
echo ""
echo "  ACTION REQUIRED:"
echo "    On your host device, RECONNECT to this controller"
echo "    from Bluetooth settings (tap the device name)."
echo ""

countdown 20 "Waiting for host to reconnect"

CONNECTED=$(get_field "$HOST_MAC" "Connected")
if [ "$CONNECTED" = "yes" ]; then
    pass "Host $HOST_MAC reconnected successfully"
else
    echo "  Device not yet connected — waiting longer..."
    countdown 15 "Extended wait for reconnect"
    CONNECTED=$(get_field "$HOST_MAC" "Connected")
    if [ "$CONNECTED" = "yes" ]; then
        pass "Host $HOST_MAC reconnected successfully"
    else
        fail "Host $HOST_MAC failed to reconnect"
    fi
fi

dump_device "$HOST_MAC"

# Verify trust survived
TRUSTED=$(get_field "$HOST_MAC" "Trusted")
if [ "$TRUSTED" = "yes" ]; then
    pass "Host $HOST_MAC still Trusted after reconnect"
else
    fail "Host $HOST_MAC lost Trusted status after reconnect"
fi

# ==========================================================
#  STEP 4: Forget on the host, re-pair
# ==========================================================
step_banner 4 "Forget on host, then re-pair"
echo ""
echo "  ACTION REQUIRED:"
echo "    On your host device, FORGET / UNPAIR this controller"
echo "    from Bluetooth settings."
echo ""

countdown 15 "Waiting for host to forget the controller"

# The device might still show as paired on our side even after the host forgets.
# We need to remove it on our side too so BlueZ starts fresh.
# Disconnect first — remove fails if the device is still connected.
echo "  Disconnecting $HOST_MAC before removing..."
bluetoothctl disconnect "$HOST_MAC" >/dev/null 2>&1 || true
sleep 2
echo "  Removing $HOST_MAC from local BlueZ to clean up stale pairing..."
bluetoothctl remove "$HOST_MAC" >/dev/null 2>&1 || true
sleep 2

# Verify it's gone
PAIRED=$(get_field "$HOST_MAC" "Paired")
if [ "$PAIRED" != "yes" ]; then
    pass "Host $HOST_MAC removed from local pairings"
else
    fail "Host $HOST_MAC still shows as Paired after remove"
fi

echo ""
echo "  Now RE-PAIR from the host."
echo ""
echo "  ACTION REQUIRED:"
echo "    On your host device, open Bluetooth settings and pair"
echo "    with this controller again (it should appear as a new device)."
echo ""

countdown 30 "Waiting for host to re-pair"

# Look for the device again
PAIRED_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
REPAIR_MAC=""
for mac in $(echo "$PAIRED_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    REPAIR_MAC="$mac"
    break
done

if [ -z "$REPAIR_MAC" ]; then
    echo "  No paired device yet — extra time..."
    countdown 30 "Extended wait for re-pairing"
    PAIRED_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
    for mac in $(echo "$PAIRED_DEVICES" | awk '{print $2}'); do
        [ -z "$mac" ] && continue
        REPAIR_MAC="$mac"
        break
    done
fi

if [ -z "$REPAIR_MAC" ]; then
    fail "No device re-paired — cannot continue step 4 checks"
else
    echo "  Re-paired host: $REPAIR_MAC"
    dump_device "$REPAIR_MAC"
    pass "Host $REPAIR_MAC re-paired successfully"

    TRUSTED=$(get_field "$REPAIR_MAC" "Trusted")
    if [ "$TRUSTED" = "yes" ]; then
        pass "Host $REPAIR_MAC is auto-Trusted after re-pair"
    else
        fail "Host $REPAIR_MAC is NOT Trusted after re-pair (auto-trust failed)"
    fi

    CONNECTED=$(get_field "$REPAIR_MAC" "Connected")
    if [ "$CONNECTED" = "yes" ]; then
        pass "Host $REPAIR_MAC is connected after re-pair"
    else
        fail "Host $REPAIR_MAC is not connected after re-pair"
    fi
fi

# ==========================================================
#  STEP 5: Final reconnect after re-pair
# ==========================================================
step_banner 5 "Disconnect and reconnect after re-pair"

if [ -n "$REPAIR_MAC" ]; then
    echo ""
    echo "  ACTION REQUIRED:"
    echo "    On your host device, DISCONNECT the controller."
    echo ""

    countdown 15 "Waiting for host to disconnect"

    CONNECTED=$(get_field "$REPAIR_MAC" "Connected")
    if [ "$CONNECTED" != "yes" ]; then
        pass "Host $REPAIR_MAC disconnected"
    else
        countdown 10 "Extended wait for disconnect"
        CONNECTED=$(get_field "$REPAIR_MAC" "Connected")
        if [ "$CONNECTED" != "yes" ]; then
            pass "Host $REPAIR_MAC disconnected"
        else
            fail "Host $REPAIR_MAC still connected"
        fi
    fi

    echo ""
    echo "  ACTION REQUIRED:"
    echo "    On your host device, RECONNECT to the controller."
    echo ""

    countdown 20 "Waiting for host to reconnect"

    CONNECTED=$(get_field "$REPAIR_MAC" "Connected")
    if [ "$CONNECTED" = "yes" ]; then
        pass "Host $REPAIR_MAC reconnected after forget+re-pair cycle"
    else
        countdown 15 "Extended wait for reconnect"
        CONNECTED=$(get_field "$REPAIR_MAC" "Connected")
        if [ "$CONNECTED" = "yes" ]; then
            pass "Host $REPAIR_MAC reconnected after forget+re-pair cycle"
        else
            fail "Host $REPAIR_MAC failed to reconnect after forget+re-pair"
        fi
    fi

    dump_device "$REPAIR_MAC"

    TRUSTED=$(get_field "$REPAIR_MAC" "Trusted")
    if [ "$TRUSTED" = "yes" ]; then
        pass "Host $REPAIR_MAC still Trusted after full cycle"
    else
        fail "Host $REPAIR_MAC lost Trusted status after full cycle"
    fi
else
    echo "  SKIPPED — no re-paired device from step 4"
fi

# ---- Diagnostics ----
echo ""
echo "========================================"
echo "  DIAGNOSTICS"
echo "========================================"

HIDD_LOG="/var/log/hidd.log"
echo ""
echo "--- hidd log (last 200 lines from $HIDD_LOG) ---"
if [ -f "$HIDD_LOG" ]; then
    tail -n 200 "$HIDD_LOG"
else
    echo "(hidd log not found at $HIDD_LOG)"
fi
echo "--- end hidd log ---"

# ---- Summary ----
echo ""
echo "========================================"
echo "  RESULTS"
echo "========================================"
echo "  PASS: $PASS_COUNT"
echo "  FAIL: $FAIL_COUNT"
echo "========================================"

SUMMARY="test_hidd_reconnect.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"
live_msg "$SUMMARY"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
