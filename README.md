# ControllerOS

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

## Bluetooth pairing scripts

- Enable pairing mode:
```bash
./scripts/bt_pairing_mode.sh
```
- Expected result in `bluetoothctl show` output:
  - `Pairable: yes`
  - `Discoverable: yes`

- Show adapter status and paired devices:
```bash
./scripts/bt_show_status.sh
```
- Expected result:
  - Adapter section with `Powered`, `Pairable`, and `Discoverable`
  - `paired-devices` listing known host device entries

- Capture debug logs during pairing attempt:
```bash
./scripts/bt_debug_pairing_capture.sh
```
- Behavior:
  - Stops current `bluetoothd` service
  - Starts `bluetoothd` in debug mode
  - Enables `Powered`, `Pairable`, and `Discoverable`
  - Writes logs to `scripts/bt_debug_YYYYMMDD_HHMMSS.log`
  - Restarts init-managed `bluetoothd` when you stop with `Ctrl+C`

- Run checkpoint 02 self-check:
```bash
./scripts/bt_checkpoint02_selfcheck.sh --require-paired
```
- Expected result:
  - `Checkpoint 02 self-check: PASS`

## Development loops (fast iteration)

Use two loops to avoid frequent full image rebuild + reboot:

- Host-only loop for `cargo fmt` / `clippy` / `test`
- Live Deck update loop using local HTTP payload hosting

Guide:

- `docs/dev_testing_loops.md`

Host helper scripts:

```bash
./scripts/dev_stage_payload.sh --hidd <path> --controllerosctl <path> [--hid-config <path>]
./scripts/dev_http_serve.sh --dir out/dev-payload --port 8000
```

Deck-side scripts included in the ISO image:

- `controlleros-dev-update`
- `controlleros-dev-list`

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

## HID self-test command (checkpoint 03 step 5)

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
- `descriptor_len=<n>`
- `report_len=<n>`
- `UHID OK`
- `pattern_run=OK duration=<s>s`
