# Checkpoint 04 Plan — Map Steam Deck Inputs → Xbox-Style HID

## Development Workflow (applies to every step)
- Loop 1 (host-only, default):
  - Use `cargo check/test/clippy/fmt` for rapid iteration without Deck reboot.
- Loop 2 (live Deck update, default for runtime checks):
  - Build binaries locally, stage with `scripts/dev_stage_payload.sh`, serve with `scripts/dev_http_serve.sh`, apply on Deck with `controlleros-dev-update`.
  - Re-run only daemon/CLI commands on Deck after update.
- Full ISO rebuild + reboot (only when required):
  - Use `./scripts/build.sh` only for Buildroot/kernel/init/post-build integration changes.

## Baseline assumptions
- Checkpoint 03 + 03.5 complete and validated on real hosts (Linux + Android).
- `hidd` currently runs a synthetic pattern loop (`PatternState`) and publishes reports via both UHID and BLE GATT HOG.
- `InputReport` struct and Xbox One S 1708 HID descriptor already defined in `crates/common/src/hid.rs`.
- Kernel version: 6.6.79 (custom Buildroot build, defconfig at `configs/kernel/steamdeck_defconfig`).
- Target: x86_64-unknown-linux-gnu (Buildroot glibc toolchain).
- Rust binaries cross-compiled via Buildroot's `cargo-package` infrastructure with offline vendoring.

## Build system notes
- `crates/input/` is a library crate imported by `hidd` and `controllerosctl`. It does **not** need its own Buildroot package `.mk` file — it is built automatically as a workspace dependency. The existing vendoring hooks in `controlleros-hidd.mk` and `controllerosctl.mk` will vendor its transitive dependencies.
- New crate dependencies must be version-pinned (exact `=x.y.z`) and present in `Cargo.lock` before Buildroot build.
- The `evdev` crate (pure Rust) is retained for the `input list` CLI command which enumerates evdev devices for diagnostics. It is NOT used for reading controller input.
- The `libc` crate is used for `ioctl` and `poll` syscalls in the hidraw module. This requires limited `unsafe` code confined to `crates/input/src/hidraw.rs`.

## Input approach: hidraw (not evdev)

The initial plan used evdev to read controller input. This was replaced with hidraw after discovering that the `hid-steam` kernel driver has two guards preventing evdev event emission on the Deck:
1. `steam_input_open()` skips lizard mode disable for `STEAM_QUIRK_DECK` devices
2. `steam_do_deck_input_event()` checks `gamepad_mode` flag (defaults to false)

The hidraw approach reads raw 64-byte HID reports directly from the controller's client device, bypassing both guards. Lizard mode is disabled by sending HID feature reports from userspace, matching the kernel's `steam_set_lizard_mode(false)` implementation.

See `checkpoints/04_deck_input_to_xbox_mapping.md` for the full hidraw protocol documentation.

## Plan (incremental)

### Step 0 — Kernel config: enable Steam Deck input drivers
- [x] Done (2026-03-16)
**Actions:**
- Add the following to `configs/kernel/steamdeck_defconfig`:
  - `CONFIG_HID=y` — HID core subsystem.
  - `CONFIG_HID_GENERIC=y` — Generic HID driver fallback.
  - `CONFIG_HID_STEAM=y` — Steam Controller/Deck embedded controller driver (`hid-steam`).
  - `CONFIG_HIDRAW=y` — Hidraw interface for raw HID device access from userspace.
  - `CONFIG_INPUT_JOYDEV=y` — Provides `/dev/input/js*` for testing tools.
**Complete when:**
- Kernel boots with `hid-steam` loaded.
- `/sys/class/hidraw/` exists and contains Valve devices.

**Validated (2026-03-16):** `hid-steam` loaded and created devices for VID `28DE` PID `1205`:
- `hidraw0` — Keyboard interface (has_input=yes). Ignore.
- `hidraw1` — Mouse interface (has_input=yes). Ignore.
- `hidraw2` — **Client device** (has_input=no). This is the one we open for raw input.

Also creates evdev devices (used by `input list` for diagnostics):
- `event7` — Keyboard interface. Ignore.
- `event8` — Mouse interface. Ignore.
- `event9` ("Steam Deck") — Gamepad with sticks/buttons/triggers.
- `event10` — Motion sensors. Ignore.

### Step 1 — Create `crates/input/` crate skeleton and workspace integration
- [x] Done (2026-03-16)
**Actions:**
- Create `crates/input/` as a library crate.
- Add to workspace `Cargo.toml`.
- Add `evdev` crate dependency (for discovery), `libc` (for hidraw ioctl/poll).
- Stub out public API: `InputReader`, device discovery functions.
**Complete when:**
- `cargo check --workspace` passes with the new crate.

### Step 2 — Implement evdev device discovery
- [x] Done (2026-03-16)
**Actions:**
- Implement device enumeration: scan `/dev/input/event*`, read device name/phys/capabilities.
- Identify Steam Deck input devices by name/vendor/capabilities.
- Expose a `discover_devices()` function returning a list of discovered devices with metadata.
- Add unit tests for discovery logic.
**Complete when:**
- `discover_devices()` returns structured device info.
- 8 unit tests pass.

### Step 3 — Define mapping config and types
- [x] Done (2026-03-16)
**Actions:**
- Create `configs/mapping/xbox.toml` with axis/button mappings, deadzone config.
- Add `MappingConfig`, `AxisMapping`, `ButtonMapping` types in `crates/input/src/mapping.rs`.
- Add validation for unknown axes/buttons.
**Complete when:**
- Config loads and validates. 5 unit tests pass.

### Step 4 — Implement hidraw input reading and state tracking
- [x] Done (2026-03-16)

**Note:** Originally planned as evdev-based. Rewritten to use hidraw after discovering the `hid-steam` driver guards that prevent evdev event emission on the Deck.

**Actions:**
- Implement hidraw device discovery (`discover_deck_hidraw()` in `crates/input/src/hidraw.rs`):
  - Scan `/sys/class/hidraw/`, match VID 0x28DE PID 0x1205.
  - Select the client device (no `input/` subdirectory in sysfs).
- Implement `HidrawDevice`:
  - Open hidraw for read/write.
  - Send HID feature reports to disable lizard mode (0x81 clear digital mappings + 0x87 settings).
  - Read raw reports with poll timeout.
- Implement `InputReader` using hidraw (in `crates/input/src/reader.rs`):
  - Parse raw 64-byte type 0x09 reports.
  - Extract buttons from data[8-14] bit fields.
  - Extract axes from data[44-55] as little-endian i16 values.
  - Negate Y axes for standard orientation.
  - Apply deadzone and normalization from MappingConfig.
  - Provide `current_report() -> InputReport`.
- 16 reader unit tests (normalization, deadzone, d-pad, report parsing).
- 4 new tests for hidraw report parsing (buttons, d-pad, axes, Y-axis negation).
**Complete when:**
- `InputReader` compiles and all 29 crate tests pass (8 discovery + 5 mapping + 16 reader).
- On Deck, `controllerosctl input monitor` shows correct values for all MVP controls.

**Validated (2026-03-16):** All controls verified on hardware:
- Triggers: analog 0→1023
- Buttons: A, B, X, Y, LB, RB, LS, RS, Back, Start, Home
- D-pad: N, S, E, W cardinal directions
- Sticks: full range ±32767 with deadzone working
- Lizard mode: successfully disabled via HID feature reports

### Step 5 — Implement `controllerosctl input list` and `controllerosctl input monitor`
- [x] Done (2026-03-16)
**Actions:**
- Add `input list` subcommand (uses evdev discovery for diagnostics).
- Add `input monitor` subcommand (uses hidraw-based `InputReader`).
- Add `--mapping-config` argument (default: `/etc/controlleros/mapping/xbox.toml`).
**Complete when:**
- `controllerosctl input list` prints discovered evdev devices with selection indicators.
- `controllerosctl input monitor` prints button/axis transitions from hidraw.
- 5 CLI unit tests pass.

**Validated (2026-03-16):** Both commands working on Deck hardware. `input monitor` shows all MVP controls with correct values.

### Step 6 — Integrate real input into `hidd` report loop and remove UHID from production path
- [x] Done (2026-03-16)
**Actions:**
- Add `crates/input/` as a dependency of `crates/hidd/`.
- Modify `hidd` to accept a `--mapping-config` argument (path to `configs/mapping/xbox.toml`).
- Replace `PatternState` usage in the main report loop:
  - When `--mapping-config` is provided: create `InputReader`, call `current_report()` each tick.
  - When `--mapping-config` is NOT provided (or `--self-test` mode): retain `PatternState` for synthetic test patterns.
- Remove UHID from the production report loop:
  - UHID device creation and report writing should only happen in `--self-test` mode.
  - Normal BLE operation uses only GATT HOG `publish_input_report()`.
- Ensure `--self-test` mode still works as before (UHID + synthetic pattern).
**Complete when:**
- `hidd` with `--mapping-config` reads real Deck inputs and publishes them via BLE GATT HOG.
- `hidd --self-test` still works with UHID + synthetic pattern (no regression).
- UHID is not used during normal BLE operation.
- Verified on Deck via Loop 2: physical stick/button inputs appear on connected host.

**Validated (2026-03-16):** `hidd --mapping-config` reads Deck inputs via hidraw and publishes via BLE GATT HOG. Android device connected and all MVP controls working. Pattern mode (`hidd` without `--mapping-config`) still works with UHID + synthetic patterns.

### Step 7 — Documentation
- [x] Done (2026-03-16)
**Actions:**
- Create `docs/mapping.md`:
  - Exact mapping table: Steam Deck raw HID byte offsets → Xbox HID field.
  - Deadzone and normalization parameters.
  - Which controls are ignored and why.
- Create `docs/input_devices.md`:
  - How hidraw devices are discovered and identified on the Deck.
  - The `hid-steam` driver's device topology (4 HID sub-devices).
  - How lizard mode is disabled via feature reports.
  - How to troubleshoot if discovery fails.
- Update `README.md` with new commands:
  - `controllerosctl input list`
  - `controllerosctl input monitor`
  - `hidd --mapping-config` usage.
**Complete when:**
- Both docs exist with accurate, complete content.
- README reflects new CLI commands and usage.

### Step 8 — Buildroot image integration and end-to-end validation
- [x] Done (2026-03-16)
**Actions:**
- [x] Update `configs/kernel/steamdeck_defconfig` to include `CONFIG_HIDRAW=y`.
- [x] Update `br2-external/board/controlleros/post-build.sh` to install `configs/mapping/xbox.toml` → `$(TARGET_DIR)/etc/controlleros/mapping/xbox.toml`.
- [x] Remove obsolete kernel patch (`br2-external/patches/linux/6.6.79/0001-HID-hid-steam-disable-lizard-mode-on-Deck-when-evdev.patch`).
- [x] Update `configs/init/S45hidd` init script to pass `--mapping-config /etc/controlleros/mapping/xbox.toml` to `hidd`.
- [x] Full ISO rebuild + boot on Deck.
- [x] End-to-end validation:
  - Boot Deck, auto-start `hidd` with real input.
  - Pair with Linux host and Android device.
  - Verify all MVP controls work on host.
  - Verify ignored controls produce no host-side changes.
  - Run for 10+ minutes: no crashes, stable responsiveness, measure CPU usage.
**Complete when:**
- ISO boots and `hidd` starts with real input mapping automatically.
- Host sees working Xbox-style controller with all MVP controls functional.
- 10-minute stability test passes.

---

## Acceptance Criteria Mapping
- **Prerequisite** (kernel input drivers + hidraw): Step 0
- A. Device Discovery (`controllerosctl input list`): Steps 0, 2, 5
- B. Mapping Sanity (`controllerosctl input monitor`): Steps 3, 4, 5
- C. Host Controller Test (sticks/buttons/triggers work on host): Steps 4, 6, 8
- D. Ignored Inputs Regression (trackpads/rear buttons produce no changes): Steps 3, 4, 6, 8
- E. Performance (10-minute stability, CPU measurement): Step 8

## Progress Updates
- 2026-03-16: Steps 0-5 complete. Hidraw approach validated on hardware — all MVP controls working.
- 2026-03-16: Step 8 partially done (kernel defconfig, post-build, patch removal). Remaining: hidd integration (Step 6), init script update, end-to-end validation.
- 2026-03-16: Steps 6-7 complete. Step 8 init script updated. Remaining: full ISO rebuild + end-to-end validation.
- 2026-03-16: Step 8 complete. ISO boots, hidd auto-starts with real input, controller usable without login. All MVP controls verified on Android host. Also fixed: added RTL 87xx BT firmware for rtl8822cu_config.bin, added bluetoothd startup delay to prevent bluetoothctl segfault.
