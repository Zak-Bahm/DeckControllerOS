# Bluetooth Bond Storage Persistence (Checkpoint 02)

ControllerOS persists Bluetooth bond data by mounting persistent storage and bind-mounting:

- Persistent data mount: `/var/lib/controlleros`
- Bluetooth bond path: `/var/lib/controlleros/bluetooth`
- Runtime BlueZ path: `/var/lib/bluetooth` (bind mount)

## Boot behavior

1. `S35bluetooth-storage` runs before `bluetoothd`.
2. It mounts persistent storage and bind-mounts `/var/lib/bluetooth`.
3. `S40bluetoothd` refuses to start if `/var/lib/bluetooth` is not a mountpoint.

## Default storage target

`S35bluetooth-storage` default device:
- `/dev/disk/by-label/CTRL_OS_DATA`

Default filesystem:
- `ext4`

Default wait:
- `5` seconds for device appearance (removable media timing).

## Optional override

Override file:
- `/etc/default/controlleros-storage`

Supported variables:
- `CONTROLLEROS_DATA_DEV`
- `CONTROLLEROS_DATA_FS`
- `CONTROLLEROS_DATA_WAIT_SECONDS`

Example:

```sh
CONTROLLEROS_DATA_DEV="/dev/sda4"
CONTROLLEROS_DATA_FS="ext4"
CONTROLLEROS_DATA_WAIT_SECONDS="10"
```

## Verification after boot

Check mountpoints:

```sh
mount | grep -E '/var/lib/controlleros|/var/lib/bluetooth'
```

Expected:
- device mounted on `/var/lib/controlleros`
- bind mount from `/var/lib/controlleros/bluetooth` to `/var/lib/bluetooth`

Check bonded host persistence:

1. Pair host once.
2. Reboot ControllerOS.
3. Run:
```sh
bluetoothctl devices Paired
```
4. Previously paired host should still be listed.
