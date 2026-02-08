#!/bin/sh

set -eu

if ! command -v bluetoothctl >/dev/null 2>&1; then
  echo "error: bluetoothctl not found in PATH" >&2
  exit 1
fi

if ! SHOW_OUTPUT="$(bluetoothctl show 2>/dev/null)"; then
  echo "error: no Bluetooth controller available" >&2
  exit 1
fi

echo "=== Adapter Status ==="
printf '%s\n' "$SHOW_OUTPUT"
echo
echo "=== Paired Devices ==="
bluetoothctl devices Paired
