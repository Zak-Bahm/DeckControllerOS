#!/usr/bin/env bash
# bt_pair_test.sh — host-side (Linux/BlueZ central) driver for the BLE pairing test.
#
# Pairs with the ControllerOS Deck while the deck-side capture
# (scripts/tests/test_bt_pair_capture.sh) is running, and records the central's
# view: a host btmon HCI/SMP trace plus bluetoothctl pair/connect output and a
# 1 Hz transition log of Connected/Paired/ServicesResolved.
#
# Run during the deck capture's pairing window:
#   sudo ./scripts/bt_pair_test.sh
#
# Output (under --logs-dir, default out/dev-logs):
#   host_btmon_<ts>.txt    decoded HCI/SMP trace from the host adapter
#   host_bt_pair_<ts>.log  verdict + central state + pair/connect command output
set -euo pipefail

DECK_NAME="ControllerOS Xbox Controller"
DECK_MAC=""
HCI="hci0"
WINDOW=60
LOGS_DIR="out/dev-logs"

usage() {
    cat <<EOF
Usage: sudo $0 [options]

  --deck-name <name>   Advertised name to discover (default: "$DECK_NAME")
  --deck-mac <MAC>     Target MAC directly, skipping discovery
  --hci <dev>          Local HCI adapter for btmon (default: $HCI)
  --window <seconds>   Discovery/observation window (default: $WINDOW)
  --logs-dir <dir>     Where to write host logs (default: $LOGS_DIR)
  -h, --help           Show this help
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --deck-name) DECK_NAME="$2"; shift 2 ;;
        --deck-mac)  DECK_MAC="$2"; shift 2 ;;
        --hci)       HCI="$2"; shift 2 ;;
        --window)    WINDOW="$2"; shift 2 ;;
        --logs-dir)  LOGS_DIR="$2"; shift 2 ;;
        -h|--help)   usage; exit 0 ;;
        *) echo "error: unknown argument: $1" >&2; usage >&2; exit 1 ;;
    esac
done

if ! command -v bluetoothctl >/dev/null 2>&1; then
    echo "error: bluetoothctl not found" >&2
    exit 1
fi
if ! command -v btmon >/dev/null 2>&1; then
    echo "error: btmon not found (install bluez)" >&2
    exit 1
fi

TS="$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$LOGS_DIR"
BTMON_TXT="$LOGS_DIR/host_btmon_$TS.txt"
REPORT="$LOGS_DIR/host_bt_pair_$TS.log"

PASS=0
FAIL=0
BTMON_PID=""

pass() { echo "PASS: $1" | tee -a "$REPORT"; PASS=$((PASS + 1)); }
fail() { echo "FAIL: $1" | tee -a "$REPORT"; FAIL=$((FAIL + 1)); }
log()  { echo "$1" | tee -a "$REPORT"; }

bctl() { bluetoothctl "$@" 2>&1 || true; }

dev_field() {
    # dev_field <mac> <FieldName>  (empty if absent; never fails under set -e)
    bluetoothctl info "$1" 2>/dev/null | grep "$2:" | awk '{print $2}' || true
}

cleanup() {
    [ -n "$BTMON_PID" ] && kill "$BTMON_PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

{
    echo "host_bt_pair report"
    echo "started_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "deck_name=$DECK_NAME"
    echo "hci=$HCI window=${WINDOW}s"
    echo ""
} > "$REPORT"

echo "Starting host btmon on $HCI -> $BTMON_TXT"
btmon -i "$HCI" >"$BTMON_TXT" 2>&1 &
BTMON_PID=$!
sleep 1
if kill -0 "$BTMON_PID" >/dev/null 2>&1; then
    pass "host btmon capturing on $HCI"
else
    fail "host btmon failed to start (need root?)"
fi

# Ensure the controller is powered and an agent is available for Just Works.
bctl power on >/dev/null
bctl agent on >/dev/null
bctl default-agent >/dev/null

# ---- Discover the target ----
if [ -z "$DECK_MAC" ]; then
    log ""
    log "Scanning for \"$DECK_NAME\" (up to ${WINDOW}s)..."
    deadline=$(( $(date +%s) + WINDOW ))
    while [ "$(date +%s)" -lt "$deadline" ]; do
        bctl --timeout 6 scan on >/dev/null
        # Match the advertised name in the device list.
        DECK_MAC="$( { bluetoothctl devices 2>/dev/null || true; } \
            | while IFS= read -r line; do
                  mac="$(echo "$line" | awk '{print $2}')"
                  name="$(echo "$line" | cut -d' ' -f3-)"
                  if [ "$name" = "$DECK_NAME" ]; then echo "$mac"; break; fi
              done)"
        [ -n "$DECK_MAC" ] && break
        log "  not found yet, retrying scan..."
    done
fi

if [ -z "$DECK_MAC" ]; then
    fail "could not discover \"$DECK_NAME\" within ${WINDOW}s"
    log ""
    log "=== Results: PASS=$PASS FAIL=$FAIL ==="
    exit 1
fi
pass "discovered target $DECK_MAC"

# ---- Remove any stale bond so we exercise a fresh pair ----
if bluetoothctl info "$DECK_MAC" >/dev/null 2>&1; then
    log "Removing stale bond for $DECK_MAC..."
    bctl remove "$DECK_MAC" >/dev/null
    sleep 1
fi

# ---- Pair, trust, connect ----
log ""
log "--- pair $DECK_MAC ---"
PAIR_OUT="$(bctl pair "$DECK_MAC")"
log "$PAIR_OUT"
if echo "$PAIR_OUT" | grep -qiE 'Pairing successful'; then
    pass "pair reported success"
else
    fail "pair did not report success"
fi

bctl trust "$DECK_MAC" >/dev/null

log ""
log "--- connect $DECK_MAC ---"
CONNECT_OUT="$(bctl connect "$DECK_MAC")"
log "$CONNECT_OUT"
if echo "$CONNECT_OUT" | grep -qiE 'Connection successful'; then
    pass "connect reported success"
else
    fail "connect did not report success"
fi

# ---- Observe central-side state (transitions) ----
log ""
log "Observing central-side state for ${WINDOW}s..."
prev=""
saw_resolved=0
deadline=$(( $(date +%s) + WINDOW ))
while [ "$(date +%s)" -lt "$deadline" ]; do
    c="$(dev_field "$DECK_MAC" Connected)"
    p="$(dev_field "$DECK_MAC" Paired)"
    s="$(dev_field "$DECK_MAC" ServicesResolved)"
    [ "$s" = "yes" ] && saw_resolved=1
    cur="connected=$c paired=$p resolved=$s"
    if [ "$cur" != "$prev" ]; then
        log "[$(date -u +%H:%M:%SZ)] $cur"
        prev="$cur"
    fi
    # Keep watching the whole window so a connect-then-drop is captured too.
    sleep 1
done

# ---- Verdict ----
log ""
FINAL_C="$(dev_field "$DECK_MAC" Connected)"
FINAL_S="$(dev_field "$DECK_MAC" ServicesResolved)"
if [ "$FINAL_C" = "yes" ] && [ "$FINAL_S" = "yes" ]; then
    pass "central: connected with services resolved"
else
    fail "central: not fully connected (connected=$FINAL_C resolved=$FINAL_S)"
    if [ "$saw_resolved" -eq 0 ]; then
        log "  NOTE: ServicesResolved never reached — GATT/encryption handshake failed."
    fi
fi

log ""
log "--- final bluetoothctl info $DECK_MAC ---"
bluetoothctl info "$DECK_MAC" 2>/dev/null | tee -a "$REPORT" || true

log ""
log "========================================"
log "  RESULTS: PASS=$PASS FAIL=$FAIL"
log "  host btmon trace: $BTMON_TXT"
log "  report:           $REPORT"
log "========================================"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
exit 0
