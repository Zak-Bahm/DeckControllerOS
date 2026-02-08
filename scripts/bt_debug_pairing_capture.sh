#!/bin/sh

set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
LOG_FILE="$SCRIPT_DIR/bt_debug_${TIMESTAMP}.log"
BTMON_LOG_FILE="$SCRIPT_DIR/btmon_${TIMESTAMP}.log"
INIT_SCRIPT="/etc/init.d/S40bluetoothd"
BLUETOOTHD_BIN="/usr/libexec/bluetooth/bluetoothd"
DEBUG_PID=""
BTMON_PID=""

if ! command -v bluetoothctl >/dev/null 2>&1; then
  echo "error: bluetoothctl not found in PATH" >&2
  exit 1
fi

if ! command -v btmon >/dev/null 2>&1; then
  echo "error: btmon not found in PATH" >&2
  exit 1
fi

if [ ! -x "$BLUETOOTHD_BIN" ]; then
  if command -v bluetoothd >/dev/null 2>&1; then
    BLUETOOTHD_BIN="$(command -v bluetoothd)"
  else
    echo "error: bluetoothd binary not found" >&2
    exit 1
  fi
fi

cleanup() {
  status=$?
  if [ -n "$DEBUG_PID" ] && kill -0 "$DEBUG_PID" 2>/dev/null; then
    kill "$DEBUG_PID" 2>/dev/null || true
    wait "$DEBUG_PID" 2>/dev/null || true
  fi
  if [ -n "$BTMON_PID" ] && kill -0 "$BTMON_PID" 2>/dev/null; then
    kill "$BTMON_PID" 2>/dev/null || true
    wait "$BTMON_PID" 2>/dev/null || true
  fi
  if [ -x "$INIT_SCRIPT" ]; then
    "$INIT_SCRIPT" start >/dev/null 2>&1 || true
  fi
  echo
  echo "debug session finished"
  echo "bluetoothd log file: $LOG_FILE"
  echo "btmon log file: $BTMON_LOG_FILE"
  exit "$status"
}

trap cleanup INT TERM EXIT

if [ -x "$INIT_SCRIPT" ]; then
  "$INIT_SCRIPT" stop >/dev/null 2>&1 || true
fi

echo "starting bluetoothd debug capture"
echo "bluetoothd log file: $LOG_FILE"
echo "btmon log file: $BTMON_LOG_FILE"

btmon >"$BTMON_LOG_FILE" 2>&1 &
BTMON_PID="$!"

"$BLUETOOTHD_BIN" -n -d >"$LOG_FILE" 2>&1 &
DEBUG_PID="$!"

sleep 1
if ! kill -0 "$BTMON_PID" 2>/dev/null; then
  echo "error: btmon failed to start" >&2
  exit 1
fi
if ! kill -0 "$DEBUG_PID" 2>/dev/null; then
  echo "error: bluetoothd failed to start in debug mode" >&2
  exit 1
fi

bluetoothctl power on >/dev/null
bluetoothctl pairable on >/dev/null
bluetoothctl discoverable on >/dev/null

SHOW_OUTPUT="$(bluetoothctl show)"
PAIRABLE_STATE="$(printf '%s\n' "$SHOW_OUTPUT" | sed -n 's/^[[:space:]]*Pairable:[[:space:]]*//p' | head -n1)"
DISCOVERABLE_STATE="$(printf '%s\n' "$SHOW_OUTPUT" | sed -n 's/^[[:space:]]*Discoverable:[[:space:]]*//p' | head -n1)"

printf '%s\n' "$SHOW_OUTPUT"

if [ "$PAIRABLE_STATE" != "yes" ] || [ "$DISCOVERABLE_STATE" != "yes" ]; then
  echo "error: failed to enable pairing mode (Pairable=$PAIRABLE_STATE Discoverable=$DISCOVERABLE_STATE)" >&2
  exit 1
fi

echo "pairing mode enabled; attempt pairing from host now"
echo "press Ctrl+C when finished"

while kill -0 "$DEBUG_PID" 2>/dev/null; do
  sleep 1
done

echo "error: bluetoothd debug process exited unexpectedly" >&2
exit 1
