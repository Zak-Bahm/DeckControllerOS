#!/bin/sh

set -eu

if ! command -v bluetoothctl >/dev/null 2>&1; then
  echo "error: bluetoothctl not found in PATH" >&2
  exit 1
fi

if ! bluetoothctl show >/dev/null 2>&1; then
  echo "error: no Bluetooth controller available" >&2
  exit 1
fi

bluetoothctl power on >/dev/null
bluetoothctl pairable on >/dev/null
bluetoothctl discoverable on >/dev/null

SHOW_OUTPUT="$(bluetoothctl show)"

PAIRABLE_STATE="$(printf '%s\n' "$SHOW_OUTPUT" | sed -n 's/^[[:space:]]*Pairable:[[:space:]]*//p' | head -n1)"
DISCOVERABLE_STATE="$(printf '%s\n' "$SHOW_OUTPUT" | sed -n 's/^[[:space:]]*Discoverable:[[:space:]]*//p' | head -n1)"

echo "$SHOW_OUTPUT"

if [ "$PAIRABLE_STATE" != "yes" ] || [ "$DISCOVERABLE_STATE" != "yes" ]; then
  echo "error: failed to enable pairing mode (Pairable=$PAIRABLE_STATE Discoverable=$DISCOVERABLE_STATE)" >&2
  exit 1
fi

echo "pairing mode enabled"
