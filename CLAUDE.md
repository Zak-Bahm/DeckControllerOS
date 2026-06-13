# ControllerOS — Claude Code Context

ControllerOS is a Buildroot-based Linux OS image for the Steam Deck that boots into a dedicated controller mode, exposing the Deck as a Bluetooth HID Xbox-style gamepad via BlueZ GATT HID-over-GATT Profile (HOGP) — no host software required.

See `AGENTS.md` for AI execution rules (checkpoint progression, coding standards, quality gates).

---

## Workspace layout

| Crate | Role |
|-------|------|
| `crates/common` | Shared HID report types and TOML config parsing |
| `crates/input` | evdev/hidraw device discovery and axis/button mapping |
| `crates/hidd` | BLE HID daemon — BlueZ D-Bus GATT HOG; UHID used only for `--self-test` |
| `crates/controllerosctl` | CLI diagnostics: `input list`, `input monitor`, `hid self-test` |
| `crates/gui` | Slint GUI (`deck` feature default; `desktop` feature for host preview) |

---

## Rust quality gates (run before every Deck update)

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## Full image build

Required when changing: `configs/kernel/`, `configs/buildroot/`, `br2-external/`, `configs/init/`, or anything installed by `post-build.sh`.

```sh
./scripts/build.sh              # incremental
./scripts/build.sh --clean-rust # force Rust recompile without full re-vendor
```

Output: `out/controlleros.iso`. See `README.md` for Buildroot manual steps and Ventoy boot instructions.

---

## Fast dev loop — no rebuild or reboot

For Rust binary and config-only changes:

**1. Build binaries on host:**
```sh
cargo build --release -p hidd -p controllerosctl
```

**2. Stage payload:**
```sh
./scripts/dev_stage_payload.sh \
  --hidd target/release/hidd \
  --controllerosctl target/release/controllerosctl \
  [--gui target/release/controlleros-gui] \
  [--hid-config configs/hid/hid.toml]
```
Output: `out/dev-payload/` with `manifest.txt` + binaries + optional configs.
Staging `--gui` deploys `controlleros-gui` to the Deck (installed to `/usr/bin/`).

**3. Serve over HTTP:**
```sh
./scripts/dev_http_serve.sh \
  --dir out/dev-payload \
  --logs-dir out/dev-logs \
  --port 8000
```

**4. On Deck — pull and apply:**
```sh
controlleros-dev-update --base-url http://<HOST_IP>:8000
controlleros-dev-list   # verify deployed files
```

Deployed files land at `/var/lib/controlleros/dev/bin/` and `/var/lib/controlleros/dev/configs/`.

---

## GUI desktop preview

```sh
./scripts/dev_gui_preview.sh
# or: cargo run -p controlleros-gui --no-default-features --features desktop
```

---

## Config file locations

| Host path | Deck image path |
|-----------|----------------|
| `configs/hid/hid.toml` | `/etc/controlleros/hid.toml` |
| `configs/mapping/xbox.toml` | `/etc/controlleros/mapping/xbox.toml` |
| `configs/bluez/main.conf` | `/etc/bluetooth/main.conf` |
| `configs/bluez/input.conf` | `/etc/bluetooth/input.conf` |
| `configs/init/S*` | `/etc/init.d/` |

---

## On-Deck diagnostics

All tools below are installed in the ControllerOS image.

```sh
# Quick health check
controlleros-dev-debug all

# Bluetooth
controlleros-dev-debug bt-status      # adapter state + paired devices
controlleros-dev-debug bt-pairing     # enable discoverable + pairable mode
controlleros-dev-debug bt-scan        # scan for nearby BLE devices (10 s)
controlleros-dev-debug bt-info <MAC>
controlleros-dev-debug bt-remove <MAC or name>

# HID daemon
controlleros-dev-debug hidd-status
controlleros-dev-debug hidd-log [N]   # last N lines of /var/log/hidd.log
controlleros-dev-debug hidd-restart
controlleros-dev-debug hidd-run       # run in foreground (stops service first)

# Networking
controlleros-dev-debug net-setup <HOST_IP>   # bootstrap networking to reach dev host

# Self-test
controlleros-dev-debug self-test
```

**Remote command + log capture from host** (output POSTed to dev server):
```sh
controlleros-dev-run --base-url http://<HOST_IP>:8000 "<shell command>"
controlleros-dev-run --base-url http://<HOST_IP>:8000 --timeout-seconds 10 "<command>"

# Run a staged test script (from scripts/tests/) on the Deck — see docs/dev_testing_loops.md
controlleros-dev-run --base-url http://<HOST_IP>:8000 --shell-script test_hidraw.sh
```
Logs stored under `out/dev-logs/` on the host.

---

## Validation commands (host-side)

```sh
# Validate HID config
cargo run -p hidd -- --validate-config --config configs/hid/hid.toml

# HID self-test (requires /dev/uhid — run on Deck or in a VM with uhid module)
cargo run -p controllerosctl -- hid self-test \
  --hidd target/debug/hidd \
  --config configs/hid/hid.toml
```

---

## Key documentation

| File | Contents |
|------|----------|
| `docs/dev_testing_loops.md` | Full Loop 1/2 guide with all commands |
| `docs/mapping.md` | Deck → Xbox byte-level mapping table |
| `docs/input_devices.md` | hidraw discovery, hid-steam driver, lizard mode |
| `docs/hid_profile.md` | Xbox One S 1708 BLE profile, report formats |
| `docs/storage.md` | Bluetooth bond persistence, partition layout |
| `docs/pairing.md` | BLE pairing flow and host-side steps |
| `docs/bt_pairing_test.md` | End-to-end pairing test harness (deck + host capture) for debugging pairing failures |
| `AGENTS.md` | AI execution rules: checkpoint progression, coding standards |
| `project.md` | Project goals, constraints, architecture |
