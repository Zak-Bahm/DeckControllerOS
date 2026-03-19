# ControllerOS

Thanks to https://github.com/Mystfit/ESP32-BLE-CompositeHID for a great reference that helped a lot with getting the Xbox bluetooth setup working

## Buildroot pin

This repo vendors Buildroot as a submodule pinned to tag `2025.11` (commit `08d71521d3`).

## Build prerequisites

- Standard Buildroot host dependencies (see Buildroot manual for your distro)
- `git` with submodule support
- `make`, `gcc`, `g++`, `python3`, `tar`, `xz`, `rsync`

## Build output layout

All Buildroot output is placed under `out/buildroot` using the out-of-tree build option (`O=out/buildroot`).
Generated images are copied to `out/` for convenience.
`scripts/build.sh` automatically performs a clean rebuild when the effective Buildroot config changes to avoid stale package configuration.

## Build (manual)

1) Configure Buildroot (defconfig):
```bash
make -C buildroot O=../out/buildroot BR2_EXTERNAL=../br2-external \
  BR2_DEFCONFIG=../configs/buildroot/controlleros_defconfig defconfig
```

2) Build:
```bash
make -C buildroot O=../out/buildroot BR2_EXTERNAL=../br2-external
```

3) Copy/rename ISO for Ventoy:
```bash
cp out/buildroot/images/rootfs.iso9660 out/buildroot/images/controlleros.iso
```

## Boot on Steam Deck (Ventoy)

1) Create a Ventoy USB stick.
2) Copy `out/buildroot/images/controlleros.iso` to the Ventoy USB root.
3) Power off the Steam Deck.
4) Hold Volume Down and press Power to open the boot manager.
5) Select the Ventoy USB device.
6) Use Ventoy's GRUB2 mode and select `controlleros.iso`.
7) Successful boot shows a ControllerOS login prompt.

## Local terminals on Deck

The image provides login gettys on `tty1`, `tty2`, and `tty3`.

- Switch consoles with:
  - `chvt 1`
  - `chvt 2`
  - `chvt 3`
- On keyboards where VT hotkeys are supported, `Ctrl+Alt+F1/F2/F3` may also work.

## On-Deck debugging (controlleros-dev-debug)

The `controlleros-dev-debug` tool is included in the ISO for development.
It consolidates Bluetooth and hidd diagnostics into a single command.

```bash
# Quick health check (adapter state + hidd status + recent logs)
controlleros-dev-debug all

# Enable pairing mode (discoverable + pairable + NoInputNoOutput agent)
controlleros-dev-debug bt-pairing

# Show adapter state and paired/connected devices
controlleros-dev-debug bt-status

# Show hidd process status and recent log output
controlleros-dev-debug hidd-status

# View last N lines of hidd log (default 30)
controlleros-dev-debug hidd-log 50

# Run hidd in foreground to see live output (stops the service first)
controlleros-dev-debug hidd-run

# Restart the hidd service
controlleros-dev-debug hidd-restart

# Run the HID self-test
controlleros-dev-debug self-test

# Scan for nearby BLE devices (10s)
controlleros-dev-debug bt-scan

# Show detailed info / remove a paired device
controlleros-dev-debug bt-info <MAC>
controlleros-dev-debug bt-remove <MAC or name>
```

## Host-side validation (checkpoint 03+)

Run from this repo on a Linux host:
```bash
./scripts/bt_checkpoint03_host_validate.sh
```
- Default target name: `ControllerOS Xbox Controller` (from `configs/bluez/main.conf`).
- Logs written to `out/host-logs/`.
- Expected result:
  - `PASS: Host discovered, paired, trusted, connected, and observed changing input pattern`

## Development loops (fast iteration)

Use two loops to avoid frequent full image rebuild + reboot:

- Host-only loop for `cargo fmt` / `clippy` / `test`
- Live Deck update loop using local HTTP payload hosting

Guide:

- `docs/dev_testing_loops.md`

Host helper scripts:

```bash
./scripts/dev_stage_payload.sh --hidd <path> --controllerosctl <path> [--hid-config <path>]
./scripts/dev_http_serve.sh --dir out/dev-payload --logs-dir out/dev-logs --port 8000
```

Deck-side scripts included in the ISO image:

- `controlleros-dev-update`
- `controlleros-dev-list`
- `controlleros-dev-run`
- `controlleros-dev-debug`

Upload Deck command logs to host during Loop 2:

```bash
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 "bluetoothctl show && /etc/init.d/S45hidd status"
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --timeout-seconds 10 "/var/lib/controlleros/dev/bin/hidd"
```

This sends full command output (`stdout` + `stderr`) to the dev server log
endpoint (`/logs`) and stores logs under `out/dev-logs/` on the host.
If `--timeout-seconds` is provided, the command is terminated after that many
seconds and returns exit code `124`.

## HID config validation (checkpoint 03)

Validate the HID runtime config from the host workspace:

```bash
cargo run -p hidd -- --validate-config --config configs/hid/hid.toml
```

Expected result:
- Prints `HID config OK: ...`
- Exits with code `0`

Failure behavior:
- Invalid config prints a clear `hidd: invalid HID config: ...` error
- Exits non-zero

## HID self-test command (checkpoint 03 step 7)

Run from host workspace (uses local binaries):

```bash
cargo run -p controllerosctl -- \
  hid self-test \
  --hidd target/debug/hidd \
  --config configs/hid/hid.toml
```

Run on Deck after Dev Loop 2 update:

```bash
/var/lib/controlleros/dev/bin/controllerosctl \
  hid self-test \
  --hidd /var/lib/controlleros/dev/bin/hidd \
  --config /var/lib/controlleros/dev/configs/hid/hid.toml
```

Expected success output includes:
- `profile_mode=<mode>`
- `profile_identity=vid=0x.... pid=0x.... version=0x.... country=<n>`
- `descriptor_len=<n>`
- `report_len=<n>`
- `UHID OK`
- `pattern_run=OK duration=<s>s`

Profile details:
- `docs/hid_profile.md`

## Input mapping (checkpoint 04)

The Steam Deck's physical controls are read via hidraw and mapped to Xbox One S-style HID reports transmitted over BLE.

### CLI commands

List detected input devices (evdev-based diagnostics):
```bash
controllerosctl input list
```

Monitor live controller input from hidraw (mapped values):
```bash
controllerosctl input monitor --mapping-config /etc/controlleros/mapping/xbox.toml
```

### hidd with real input

Start `hidd` with real Deck input (production mode):
```bash
hidd --config /etc/controlleros/hid.toml --mapping-config /etc/controlleros/mapping/xbox.toml
```

Without `--mapping-config`, `hidd` runs in pattern mode (synthetic test patterns via UHID + BLE).

### Documentation

- `docs/mapping.md` — exact byte-level mapping table, normalization, ignored controls
- `docs/input_devices.md` — hidraw discovery, hid-steam topology, lizard mode, troubleshooting

## Buildroot image integration (checkpoint 03 step 6)

`./scripts/build.sh` now installs these artifacts into the image:
- `/usr/bin/hidd`
- `/usr/bin/controllerosctl`
- `/etc/controlleros/hid.toml`
- `/etc/controlleros/mapping/xbox.toml`

All icons used are sourced from [GOFOX](https://www.flaticon.com/authors/gofox)