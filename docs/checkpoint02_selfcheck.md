# Checkpoint 02 Self-Check

Use this checklist to validate:
- adapter health (`Powered`, `Pairable`, `Discoverable`)
- host pairing visibility
- bond persistence after reboot

## 1) Pre-pairing health check

```sh
./scripts/bt_pairing_mode.sh
./scripts/bt_checkpoint02_selfcheck.sh
```

Expected:
- `Checkpoint 02 self-check: PASS`

## 2) Pair a host

From host `bluetoothctl`:

```sh
scan on
pair <CONTROLLEROS_MAC>
trust <CONTROLLEROS_MAC>
```

Then on ControllerOS:

```sh
./scripts/bt_checkpoint02_selfcheck.sh --require-paired
```

Expected:
- pass result
- paired device count greater than zero

## 3) Reboot persistence check

Reboot ControllerOS, then run:

```sh
./scripts/bt_checkpoint02_selfcheck.sh --require-paired
```

If you want to assert one specific host:

```sh
./scripts/bt_checkpoint02_selfcheck.sh --expect-mac <HOST_MAC> --require-paired
```

Expected:
- pass result after reboot
- previously paired host is still present
