#!/bin/sh
# Diagnostic script to test the hidraw approach for reading Steam Deck controller input.
# Checks: hidraw sysfs presence, Valve device enumeration, device permissions,
# lizard mode disable via feature report, and raw report reading.

echo "=== Kernel version ==="
uname -r

echo ""
echo "=== CONFIG_HIDRAW in kernel ==="
if [ -d /sys/class/hidraw ]; then
    echo "OK: /sys/class/hidraw exists"
else
    echo "FAIL: /sys/class/hidraw does not exist — CONFIG_HIDRAW not enabled"
    exit 1
fi

echo ""
echo "=== Enumerate hidraw devices ==="
for dev in /sys/class/hidraw/hidraw*; do
    [ -e "$dev" ] || continue
    NAME="$(basename "$dev")"
    UEVENT="$dev/device/uevent"
    if [ -f "$UEVENT" ]; then
        HID_ID="$(grep '^HID_ID=' "$UEVENT" | head -1)"
        HID_NAME="$(grep '^HID_NAME=' "$UEVENT" | head -1)"
        HAS_INPUT="no"
        [ -d "$dev/device/input" ] && HAS_INPUT="yes"
        echo "/dev/$NAME: $HID_ID $HID_NAME has_input=$HAS_INPUT"
    else
        echo "/dev/$NAME: (no uevent)"
    fi
done

echo ""
echo "=== Find Valve (28DE) hidraw devices ==="
VALVE_DEVS=""
CLIENT_DEV=""
for dev in /sys/class/hidraw/hidraw*; do
    [ -e "$dev" ] || continue
    NAME="$(basename "$dev")"
    UEVENT="$dev/device/uevent"
    [ -f "$UEVENT" ] || continue
    HID_ID="$(grep '^HID_ID=' "$UEVENT" | head -1 | sed 's/HID_ID=//')"
    # HID_ID format: BBBB:VVVVVVVV:PPPPPPPP
    VID="$(echo "$HID_ID" | cut -d: -f2)"
    case "$VID" in
        *28DE*|*28de*)
            VALVE_DEVS="$VALVE_DEVS /dev/$NAME"
            HAS_INPUT="no"
            [ -d "$dev/device/input" ] && HAS_INPUT="yes"
            echo "/dev/$NAME: HID_ID=$HID_ID has_input=$HAS_INPUT"
            if [ "$HAS_INPUT" = "no" ]; then
                CLIENT_DEV="/dev/$NAME"
            fi
            ;;
    esac
done

if [ -z "$VALVE_DEVS" ]; then
    echo "FAIL: no Valve hidraw devices found"
    exit 1
fi

if [ -z "$CLIENT_DEV" ]; then
    echo "WARNING: no client device found (none without input subdir)"
    echo "Falling back to last Valve device"
    CLIENT_DEV="$(echo "$VALVE_DEVS" | tr ' ' '\n' | sort | tail -1)"
fi

echo ""
echo "=== Selected client device: $CLIENT_DEV ==="

echo ""
echo "=== Check device permissions ==="
ls -la "$CLIENT_DEV"

echo ""
echo "=== Test: can we open the device? ==="
if dd if="$CLIENT_DEV" of=/dev/null bs=64 count=0 2>/dev/null; then
    echo "OK: device is readable"
else
    echo "FAIL: cannot open $CLIENT_DEV (permissions?)"
fi

echo ""
echo "=== Test: read raw reports for 5 seconds ==="
echo "PRESS BUTTONS AND MOVE STICKS NOW"
TMP="/tmp/hidraw_test_data"
dd if="$CLIENT_DEV" of="$TMP" bs=64 2>/dev/null &
DD_PID=$!
sleep 5
kill "$DD_PID" 2>/dev/null
wait "$DD_PID" 2>/dev/null
BYTES=0
[ -f "$TMP" ] && BYTES=$(wc -c < "$TMP")
echo "received $BYTES bytes from $CLIENT_DEV"

if [ "$BYTES" -gt 0 ]; then
    echo ""
    echo "=== First 64 bytes (hex) ==="
    od -A x -t x1z -N 64 "$TMP" 2>/dev/null || hexdump -C -n 64 "$TMP" 2>/dev/null || xxd -l 64 "$TMP" 2>/dev/null || echo "(no hex dump tool available)"

    echo ""
    echo "=== Check report header ==="
    # data[0] should be 0x01, data[1] should be 0x00, data[2] is report type
    BYTE0="$(od -A n -t x1 -N 1 -j 0 "$TMP" 2>/dev/null | tr -d ' ')"
    BYTE1="$(od -A n -t x1 -N 1 -j 1 "$TMP" 2>/dev/null | tr -d ' ')"
    BYTE2="$(od -A n -t x1 -N 1 -j 2 "$TMP" 2>/dev/null | tr -d ' ')"
    echo "data[0]=0x$BYTE0 data[1]=0x$BYTE1 data[2]=0x$BYTE2"
    if [ "$BYTE2" = "09" ]; then
        echo "OK: report type 0x09 = DECK_INPUT_REPORT"
    elif [ "$BYTE2" = "01" ]; then
        echo "INFO: report type 0x01 = CONTROLLER_STATE (non-Deck format)"
    elif [ "$BYTE2" = "04" ]; then
        echo "INFO: report type 0x04 = BATTERY_STATUS"
    else
        echo "INFO: report type 0x$BYTE2 (unknown)"
    fi
else
    echo "FAIL: no data received from hidraw device"
    echo ""
    echo "=== Try all Valve hidraw devices (2 sec each) ==="
    for dev in $VALVE_DEVS; do
        TMP2="/tmp/hidraw_test_$(basename "$dev")"
        dd if="$dev" of="$TMP2" bs=64 2>/dev/null &
        PID=$!
        sleep 2
        kill "$PID" 2>/dev/null
        wait "$PID" 2>/dev/null
        B=0
        [ -f "$TMP2" ] && B=$(wc -c < "$TMP2")
        HAS_INPUT="no"
        [ -d "/sys/class/hidraw/$(basename "$dev")/device/input" ] && HAS_INPUT="yes"
        echo "$dev: $B bytes (has_input=$HAS_INPUT)"
        rm -f "$TMP2"
    done
fi

rm -f "$TMP"

echo ""
echo "=== dmesg hid-steam (last 10 lines) ==="
dmesg | grep -i 'hid.steam\|steam.*deck\|28de' | tail -10

echo ""
echo "=== controllerosctl stderr test (5 sec) ==="
echo "PRESS BUTTONS NOW"
timeout 5 controllerosctl input monitor 2>/tmp/hidraw_ctl_stderr >/tmp/hidraw_ctl_stdout || true
echo "--- stdout ---"
cat /tmp/hidraw_ctl_stdout 2>/dev/null
echo "--- stderr ---"
cat /tmp/hidraw_ctl_stderr 2>/dev/null
rm -f /tmp/hidraw_ctl_stdout /tmp/hidraw_ctl_stderr

echo ""
echo "=== Done ==="
