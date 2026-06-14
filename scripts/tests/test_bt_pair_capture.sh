#!/bin/sh
# test_bt_pair_capture.sh — capture a full BLE pairing attempt from the Deck
# (peripheral) side for debugging pairing failures.
#
# Symptom this targets: a host sees the controller and tries to connect, the
# connection fails, and the controller then stops advertising (disappears).
#
# This script drives the pairing process from a clean slate and captures
# EVERYTHING needed to diagnose why a pair attempt fails:
#   - btmon HCI/SMP wire trace (text)
#   - verbose bluetoothd -d log (SMP / authentication failure reasons)
#   - full hidd log
#   - a 1 Hz transition log of adapter Discoverable + per-device
#     Paired/Trusted/Connected/ServicesResolved (pinpoints when the controller
#     stops being visible and whether the GATT handshake ever completed)
#   - dmesg bluetooth/hci/smp lines
#
# Run on Deck via:
#   cosd-run --base-url http://<DEV_MACHINE_IP>:8000 \
#       --timeout-seconds 120 --shell-script test_bt_pair_capture.sh
#
# Pair from a Linux/BlueZ host during the window with:
#   sudo ./scripts/bt_pair_test.sh
#
# Optional env:
#   WINDOW_SECONDS   length of the pairing observation window (default 60)

PASS_COUNT=0
FAIL_COUNT=0

WINDOW_SECONDS="${WINDOW_SECONDS:-60}"
CAP_DIR="/tmp/bt_pair_capture"
BTMON_TXT="$CAP_DIR/btmon.txt"
BTD_LOG="$CAP_DIR/bluetoothd.log"
TRANS_LOG="$CAP_DIR/transitions.log"
HIDD_LOG="/var/log/hidd.log"
BLUETOOTHD_BIN="/usr/libexec/bluetooth/bluetoothd"

BTMON_PID=""
BTD_PID=""

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

now_ts() {
    date -u +%H:%M:%SZ 2>/dev/null || echo "??:??:??"
}

device_info() {
    bluetoothctl info "$1" 2>/dev/null
}

get_field() {
    # get_field <mac> <FieldName> -> value (e.g. get_field AA:BB:CC Trusted -> yes)
    device_info "$1" | grep "$2:" | awk '{print $2}'
}

adapter_field() {
    # adapter_field <FieldName> -> value from `bluetoothctl show`
    bluetoothctl show 2>/dev/null \
        | sed -n "s/^[[:space:]]*$1:[[:space:]]*//p" | head -n1
}

# Restore normal services on any exit so the Deck returns to a working state.
restore_services() {
    echo ""
    echo "  Restoring normal Bluetooth services..."
    [ -n "$BTMON_PID" ] && kill "$BTMON_PID" >/dev/null 2>&1
    if [ -n "$BTD_PID" ]; then
        kill "$BTD_PID" >/dev/null 2>&1
        # Wait for the manual bluetoothd to release the HCI/D-Bus name.
        i=0
        while kill -0 "$BTD_PID" >/dev/null 2>&1 && [ "$i" -lt 20 ]; do
            sleep 0.2
            i=$((i + 1))
        done
    fi
    [ -x /etc/init.d/S40bluetoothd ] && /etc/init.d/S40bluetoothd start >/dev/null 2>&1
    sleep 1
    [ -x /etc/init.d/S41bluetooth-power ] && /etc/init.d/S41bluetooth-power >/dev/null 2>&1
    [ -x /etc/init.d/S45hidd ] && /etc/init.d/S45hidd start >/dev/null 2>&1
}

cleanup() {
    restore_services
}
trap cleanup EXIT INT TERM

live_msg "Starting test_bt_pair_capture.sh"

# ==========================================================
#  STEP 0: Prerequisites
# ==========================================================
step_banner 0 "Prerequisites"

ABORT=0
for tool in bluetoothctl btmon; do
    if command -v "$tool" >/dev/null 2>&1; then
        pass "$tool available"
    else
        fail "$tool not found"
        ABORT=1
    fi
done

if [ -x "$BLUETOOTHD_BIN" ]; then
    pass "bluetoothd found at $BLUETOOTHD_BIN"
else
    fail "bluetoothd not found at $BLUETOOTHD_BIN"
    ABORT=1
fi

if [ -x /usr/bin/hidd ] || [ -x /var/lib/controlleros/dev/bin/hidd ]; then
    pass "hidd binary present"
else
    fail "hidd binary not found"
    ABORT=1
fi

if [ "$ABORT" -ne 0 ]; then
    echo "=== Results: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
    exit 1
fi

mkdir -p "$CAP_DIR"
: > "$BTMON_TXT"
: > "$BTD_LOG"
: > "$TRANS_LOG"

# ==========================================================
#  STEP 1: Clean slate
# ==========================================================
step_banner 1 "Clean slate (remove bonds, stop services)"

echo "  Removing previously paired devices..."
OLD_DEVICES=$(bluetoothctl devices Paired 2>/dev/null || bluetoothctl devices 2>/dev/null)
for mac in $(echo "$OLD_DEVICES" | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    echo "    removing $mac"
    bluetoothctl remove "$mac" >/dev/null 2>&1 || true
done

echo "  Stopping hidd..."
[ -x /etc/init.d/S45hidd ] && /etc/init.d/S45hidd stop >/dev/null 2>&1 || true
echo "  Truncating $HIDD_LOG..."
: > "$HIDD_LOG" 2>/dev/null || true

echo "  Powering adapter off to clear stale HCI advertising state..."
bluetoothctl power off >/dev/null 2>&1 || true
sleep 1
echo "  Stopping system bluetoothd service..."
[ -x /etc/init.d/S40bluetoothd ] && /etc/init.d/S40bluetoothd stop >/dev/null 2>&1 || true
sleep 1

# ==========================================================
#  STEP 2: Start verbose bluetoothd + btmon
# ==========================================================
step_banner 2 "Start verbose bluetoothd and btmon"

echo "  Launching bluetoothd -n -d (verbose) -> $BTD_LOG"
"$BLUETOOTHD_BIN" -n -d >"$BTD_LOG" 2>&1 &
BTD_PID=$!

# Wait for the D-Bus interface to come up (mirrors S41bluetooth-power).
READY=0
i=0
while [ "$i" -lt 15 ]; do
    if bluetoothctl show >/dev/null 2>&1; then
        READY=1
        break
    fi
    sleep 1
    i=$((i + 1))
done
if [ "$READY" -eq 1 ]; then
    pass "verbose bluetoothd is up (pid=$BTD_PID)"
else
    fail "verbose bluetoothd did not expose D-Bus interface"
    echo "=== Results: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
    exit 1
fi

echo "  Starting btmon -> $BTMON_TXT"
btmon >"$BTMON_TXT" 2>&1 &
BTMON_PID=$!
sleep 1
if kill -0 "$BTMON_PID" >/dev/null 2>&1; then
    pass "btmon is capturing (pid=$BTMON_PID)"
else
    fail "btmon failed to start"
fi

# ==========================================================
#  STEP 3: Start hidd and confirm advertising
# ==========================================================
step_banner 3 "Start hidd and confirm advertising"

bluetoothctl power on >/dev/null 2>&1 || true
echo "  Starting hidd..."
[ -x /etc/init.d/S45hidd ] && /etc/init.d/S45hidd start >/dev/null 2>&1 || true

# Wait for hidd to power up the adapter and start advertising.
DISCOVERABLE=""
i=0
while [ "$i" -lt 15 ]; do
    DISCOVERABLE="$(adapter_field Discoverable)"
    if [ "$DISCOVERABLE" = "yes" ]; then
        break
    fi
    sleep 1
    i=$((i + 1))
done

if pidof hidd >/dev/null 2>&1; then
    pass "hidd is running"
else
    fail "hidd is NOT running"
fi

ADAPTER_MAC="$(adapter_field Controller)"
[ -z "$ADAPTER_MAC" ] && ADAPTER_MAC="$(bluetoothctl show 2>/dev/null | head -n1 | awk '{print $2}')"
ADAPTER_NAME="$(adapter_field Name)"
POWERED="$(adapter_field Powered)"
PAIRABLE="$(adapter_field Pairable)"
DISCOVERABLE="$(adapter_field Discoverable)"

echo "  Adapter:      $ADAPTER_MAC ($ADAPTER_NAME)"
echo "  Powered:      $POWERED"
echo "  Pairable:     $PAIRABLE"
echo "  Discoverable: $DISCOVERABLE"

if [ "$DISCOVERABLE" = "yes" ]; then
    pass "adapter is discoverable (advertising)"
else
    fail "adapter is NOT discoverable after hidd start"
fi

# ==========================================================
#  STEP 4: Observe the pairing window
# ==========================================================
step_banner 4 "Pairing window ($WINDOW_SECONDS s) — pair from host NOW"
echo ""
echo "  ACTION REQUIRED on the Linux host (within $WINDOW_SECONDS s):"
echo "    sudo ./scripts/bt_pair_test.sh"
echo ""
live_msg "READY: pair from host now (deck=$ADAPTER_MAC, window=${WINDOW_SECONDS}s)"

# Sample state once per second and log only transitions. This isolates the
# exact moment the controller stops being discoverable and whether any device
# ever reached ServicesResolved=yes (a completed GATT handshake).
PREV_DISC=""
PREV_DEVSTATE=""
SAW_SERVICES_RESOLVED=0
SAW_CONNECT=0

i=0
while [ "$i" -lt "$WINDOW_SECONDS" ]; do
    TS="$(now_ts)"

    CUR_DISC="$(adapter_field Discoverable)"
    if [ "$CUR_DISC" != "$PREV_DISC" ]; then
        echo "[$TS] discoverable: ${PREV_DISC:-?} -> $CUR_DISC" | tee -a "$TRANS_LOG"
        PREV_DISC="$CUR_DISC"
    fi

    # Build a compact per-device state string for connected devices.
    CONNECTED="$(bluetoothctl devices Connected 2>/dev/null | awk '{print $2}')"
    CUR_DEVSTATE=""
    for mac in $CONNECTED; do
        [ -z "$mac" ] && continue
        SAW_CONNECT=1
        P="$(get_field "$mac" Paired)"
        T="$(get_field "$mac" Trusted)"
        S="$(get_field "$mac" ServicesResolved)"
        [ "$S" = "yes" ] && SAW_SERVICES_RESOLVED=1
        CUR_DEVSTATE="$CUR_DEVSTATE $mac(paired=$P,trusted=$T,resolved=$S)"
    done
    CUR_DEVSTATE="${CUR_DEVSTATE# }"

    if [ "$CUR_DEVSTATE" != "$PREV_DEVSTATE" ]; then
        if [ -z "$CUR_DEVSTATE" ]; then
            echo "[$TS] connected devices: (none)" | tee -a "$TRANS_LOG"
        else
            echo "[$TS] connected devices: $CUR_DEVSTATE" | tee -a "$TRANS_LOG"
        fi
        PREV_DEVSTATE="$CUR_DEVSTATE"
    fi

    sleep 1
    i=$((i + 1))
done

# ==========================================================
#  STEP 5: Verdict
# ==========================================================
step_banner 5 "Verdict"

# Find a device that completed the handshake and is still connected.
GOOD_MAC=""
for mac in $(bluetoothctl devices Connected 2>/dev/null | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    if [ "$(get_field "$mac" Paired)" = "yes" ] && \
       [ "$(get_field "$mac" ServicesResolved)" = "yes" ]; then
        GOOD_MAC="$mac"
        break
    fi
done

if [ -n "$GOOD_MAC" ]; then
    pass "device $GOOD_MAC paired with services resolved and still connected"
else
    fail "no device completed pairing (paired + services resolved + connected)"
    if [ "$SAW_CONNECT" -eq 1 ] && [ "$SAW_SERVICES_RESOLVED" -eq 0 ]; then
        echo "  NOTE: a device connected but never reached ServicesResolved —"
        echo "        the GATT/encryption handshake failed mid-pairing."
    elif [ "$SAW_CONNECT" -eq 0 ]; then
        echo "  NOTE: no device ever connected during the window."
    fi
    FINAL_DISC="$(adapter_field Discoverable)"
    echo "  Final adapter Discoverable: $FINAL_DISC"
    if [ "$FINAL_DISC" != "yes" ]; then
        echo "  NOTE: adapter is no longer discoverable — the controller"
        echo "        'disappeared' (advertising was not re-registered)."
    fi
fi

# ==========================================================
#  DIAGNOSTICS (all uploaded inline)
# ==========================================================
echo ""
echo "========================================"
echo "  DIAGNOSTICS"
echo "========================================"

echo ""
echo "--- final adapter state (bluetoothctl show) ---"
bluetoothctl show 2>/dev/null || echo "(unavailable)"

echo ""
echo "--- known devices (bluetoothctl info) ---"
for mac in $(bluetoothctl devices 2>/dev/null | awk '{print $2}'); do
    [ -z "$mac" ] && continue
    echo "  === $mac ==="
    device_info "$mac" | sed 's/^/    /'
done

echo ""
echo "--- transition log ---"
cat "$TRANS_LOG" 2>/dev/null || echo "(empty)"

echo ""
echo "--- hidd log ($HIDD_LOG) ---"
if [ -f "$HIDD_LOG" ]; then
    cat "$HIDD_LOG"
else
    echo "(not found)"
fi
echo "--- end hidd log ---"

echo ""
echo "--- verbose bluetoothd log ($BTD_LOG) ---"
cat "$BTD_LOG" 2>/dev/null || echo "(empty)"
echo "--- end bluetoothd log ---"

echo ""
echo "--- btmon HCI/SMP trace ($BTMON_TXT) ---"
cat "$BTMON_TXT" 2>/dev/null || echo "(empty)"
echo "--- end btmon trace ---"

echo ""
echo "--- dmesg (bluetooth/hci/smp) ---"
dmesg 2>/dev/null | grep -iE 'blue|hci|smp' | tail -n 60 || echo "(unavailable)"

# ==========================================================
#  RESULTS
# ==========================================================
echo ""
echo "========================================"
echo "  RESULTS"
echo "========================================"
echo "  PASS: $PASS_COUNT"
echo "  FAIL: $FAIL_COUNT"
echo "========================================"

live_msg "test_bt_pair_capture.sh complete — PASS: $PASS_COUNT, FAIL: $FAIL_COUNT"

# cleanup() restores services via the EXIT trap.
if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
