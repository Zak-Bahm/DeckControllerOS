# Bluetooth Pairing (Checkpoint 02)

This checkpoint validates adapter health, pairing mode, and bonded host visibility.

## On ControllerOS

1. Enable pairing mode:
```sh
./scripts/bt_pairing_mode.sh
```

2. Confirm adapter state and paired devices:
```sh
./scripts/bt_show_status.sh
```

Expected adapter state in output:
- `Powered: yes`
- `Pairable: yes`
- `Discoverable: yes`

Expected paired device listing:
- `bluetoothctl devices Paired` prints at least one host entry after successful pairing.

## Host pairing flow (CLI)

Use `bluetoothctl` on the host:

```sh
bluetoothctl
scan on
pair <CONTROLLEROS_MAC>
trust <CONTROLLEROS_MAC>
connect <CONTROLLEROS_MAC>
info <CONTROLLEROS_MAC>
```

Notes:
- `pair` should succeed for this checkpoint.
- `connect` may not establish a usable controller profile yet in checkpoint 02.
- For verification here, the key requirement is that the host is bonded and appears in `devices Paired` on ControllerOS.

## Debug capture

If pairing fails, capture Deck-side logs with:

```sh
./scripts/bt_debug_pairing_capture.sh
```

This writes:
- `scripts/bt_debug_YYYYMMDD_HHMMSS.log`
- `scripts/btmon_YYYYMMDD_HHMMSS.log`

Stop capture with `Ctrl+C`.
