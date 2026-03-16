# Bluetooth Pairing

## How pairing works

`hidd` registers a `NoInputNoOutput` BlueZ pairing agent via D-Bus on startup.
This enables automatic "Just Works" BLE pairing — no PIN entry or confirmation
is needed on either side. The agent is tied to hidd's lifecycle and is
unregistered when hidd stops.

The adapter is configured as discoverable and pairable by `hidd` during
GATT HOG registration (`configure_adapter_for_hog` in `crates/hidd/src/hog.rs`).

## On ControllerOS

With hidd running, the Deck is automatically discoverable and pairable.
No manual steps are needed.

To verify adapter state and paired devices:
```sh
controlleros-dev-debug bt-status
```

Expected adapter state:
- `Powered: yes`
- `Pairable: yes`
- `Discoverable: yes`

To remove a paired device:
```sh
controlleros-dev-debug bt-remove <MAC or device name>
```

## Host pairing flow

### Any host (Windows, macOS, Android, iOS, Linux)
Open Bluetooth settings and pair with `ControllerOS Xbox Controller`.
The device should appear as a gamepad.

### Linux host (CLI)
```sh
bluetoothctl
scan on
pair <CONTROLLEROS_MAC>
trust <CONTROLLEROS_MAC>
connect <CONTROLLEROS_MAC>
info <CONTROLLEROS_MAC>
```

### Verify input on Linux host
```sh
sudo evtest
# Select "ControllerOS Xbox Controller"
```

## Host end-to-end validation script (Checkpoint 03+)

Run from this repo on a Linux host:

```sh
./scripts/bt_checkpoint03_host_validate.sh
```

What it validates:
- discovery of the ControllerOS target
- pair + trust + connect flow
- host-side input node appearance
- changing input events during the test-pattern window

Default target name:
- Derived from `configs/bluez/main.conf` (`Name=`).
- Current default: `ControllerOS Xbox Controller`.

Logs are written to `out/host-logs/`.

## Debug capture

If pairing fails, capture Deck-side logs with:
```sh
controlleros-dev-run \
  --base-url http://<DEV_MACHINE_IP>:8000 \
  --timeout-seconds 30 \
  "controlleros-dev-debug hidd-run"
```

Check hidd logs for agent authorization messages and any D-Bus errors.
