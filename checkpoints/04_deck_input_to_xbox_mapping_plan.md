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
- `crates/input/` does not yet exist.
- Kernel version: 6.6.79 (custom Buildroot build, defconfig at `configs/kernel/steamdeck_defconfig`).
- Target: x86_64-unknown-linux-gnu (Buildroot glibc toolchain).
- Rust binaries cross-compiled via Buildroot's `cargo-package` infrastructure with offline vendoring.

## Build system notes
- `crates/input/` is a library crate imported by `hidd` and `controllerosctl`. It does **not** need its own Buildroot package `.mk` file — it is built automatically as a workspace dependency. The existing vendoring hooks in `controlleros-hidd.mk` and `controllerosctl.mk` will vendor its transitive dependencies.
- New crate dependencies must be version-pinned (exact `=x.y.z`) and present in `Cargo.lock` before Buildroot build.
- Use the **`evdev` crate** (pure Rust, uses `libc` ioctls directly) — NOT `evdev-rs` which wraps the `libevdev` C library and would require adding `BR2_PACKAGE_LIBEVDEV=y` plus cross-compilation linking. The pure Rust `evdev` crate has no system library dependency beyond `libc` (already available).
- `#![forbid(unsafe_code)]` is valid for `crates/input/` — the `evdev` crate contains unsafe internally but exposes a safe public API.

## Plan (incremental)

### Step 0 — Kernel config: enable Steam Deck input drivers
- [x] Done (2026-03-16)
**Actions:**
- Add the following to `configs/kernel/steamdeck_defconfig`:
  - `CONFIG_HID=y` — HID core subsystem (likely auto-selected by UHID but should be explicit).
  - `CONFIG_HID_GENERIC=y` — Generic HID driver fallback for unrecognized HID devices.
  - `CONFIG_HID_STEAM=y` — Steam Controller/Deck embedded controller driver (`hid-steam`). This is the driver that creates evdev device nodes for the Deck's sticks, buttons, triggers, and d-pad. Without it, no input devices appear. Present in kernel 6.6.x at `drivers/hid/hid-steam.c`.
  - `CONFIG_INPUT_JOYDEV=y` — Optional but useful: provides `/dev/input/js*` for joystick testing tools like `jstest`.
- Requires full ISO rebuild + Deck reboot to validate.
- On Deck, verify with: `ls /dev/input/event*` and `cat /proc/bus/input/devices` to confirm Deck controller appears.
**Complete when:**
- Kernel boots with `hid-steam` loaded (check `lsmod` or `dmesg | grep -i steam`).
- At least one evdev device node exists for the Deck's gamepad controls.
- `cat /proc/bus/input/devices` shows a device with ABS and KEY capabilities matching sticks/buttons.

**Risk note:** If `hid-steam` does not claim the Deck's Neptune controller on kernel 6.6.79, check `dmesg` for HID enumeration. The Neptune controller USB VID:PID is `28DE:1205`. If the driver doesn't match, `CONFIG_HID_GENERIC=y` provides a fallback that exposes raw HID events, though axis ranges/codes may differ. Document any deviations.

**Validated (2026-03-16):** `hid-steam` loaded and created 4 input devices for VID `28DE` PID `1205`:
- `event7` — Keyboard interface (volume/power keys). Ignore.
- `event8` — Mouse interface (trackpad relative axes). Ignore.
- **`event9` ("Steam Deck")** — **Gamepad** with `js0`. ABS axes: `ABS_X`, `ABS_Y`, `ABS_RX`, `ABS_RY`, `ABS_HAT0X/Y`, `ABS_HAT1X/Y`, `ABS_HAT2X/Y`. Buttons in EV_KEY bitmask.
- `event10` — Motion sensors (gyro/accel). Ignore.

**Verified from kernel source (`drivers/hid/hid-steam.c`, `STEAM_QUIRK_DECK` path):**

Axes (gamepad device):
- `ABS_X` / `ABS_Y`: Left stick, range -32767..32767 (Y inverted by driver)
- `ABS_RX` / `ABS_RY`: Right stick, range -32767..32767 (Y inverted by driver)
- `ABS_HAT2Y`: **Left trigger** (analog), range 0..32767
- `ABS_HAT2X`: **Right trigger** (analog), range 0..32767
- `ABS_HAT0X` / `ABS_HAT0Y`: Left touchpad (ignore MVP)
- `ABS_HAT1X` / `ABS_HAT1Y`: Right touchpad (ignore MVP)

Buttons (MVP):
- `BTN_A`(0x130), `BTN_B`(0x131), `BTN_X`(0x133), `BTN_Y`(0x134)
- `BTN_TL`(0x136)/LB, `BTN_TR`(0x137)/RB
- `BTN_SELECT`(0x13a)/Back, `BTN_START`(0x13b)/Start
- `BTN_THUMBL`(0x13d)/LS click, `BTN_THUMBR`(0x13e)/RS click
- `BTN_DPAD_UP/DOWN/LEFT/RIGHT`(0x220-0x223) — **D-pad is buttons, NOT hat axes**

Buttons (ignore MVP): `BTN_TL2`/`BTN_TR2` (digital trigger), `BTN_MODE` (Steam logo), `BTN_THUMB`/`BTN_THUMB2` (pad touch), `BTN_TRIGGER_HAPPY1-4` (grips), `BTN_GEAR_DOWN`/`BTN_GEAR_UP` (back levers), `BTN_BASE` (quick access).

### Step 1 — Create `crates/input/` crate skeleton and workspace integration
- [x] Done (2026-03-16)
**Actions:**
- Create `crates/input/` as a library crate.
- Add to workspace `Cargo.toml`.
- Enforce `#![forbid(unsafe_code)]`.
- Add `evdev` crate dependency (pure Rust, pin exact version).
- Stub out public API: `InputReader`, `InputEvent`, device discovery functions.
**Complete when:**
- `cargo check --workspace` passes with the new crate.
- Verified through Loop 1 only.

### Step 2 — Implement evdev device discovery
- [x] Done (2026-03-16)
**Actions:**
- Implement device enumeration: scan `/dev/input/event*`, read device name/phys/capabilities via evdev ioctls.
- Identify Steam Deck input devices by name/vendor/capabilities (not hardcoded event numbers).
  - Primary match: look for the `hid-steam` driver device (name contains "Steam" or "Valve", VID `0x28DE`).
  - Fallback match: any device providing both `EV_ABS` (sticks/triggers) and `EV_KEY` (buttons) with the expected axis codes.
  - Expected evdev codes from `hid-steam` on Deck: `ABS_X`, `ABS_Y` (left stick), `ABS_RX`, `ABS_RY` (right stick), `ABS_HAT2X`/`ABS_HAT2Y` or `ABS_Z`/`ABS_RZ` (triggers), `ABS_HAT0X`/`ABS_HAT0Y` (d-pad), `BTN_SOUTH`/`BTN_EAST`/`BTN_NORTH`/`BTN_WEST` (ABXY), `BTN_TL`/`BTN_TR` (bumpers), `BTN_START`/`BTN_SELECT`.
  - **Important:** Verify actual evdev codes on Deck hardware in Step 0 using `evtest` or `cat /proc/bus/input/devices`. The `hid-steam` driver may use non-standard codes — document any differences.
- Expose a `discover_devices()` function returning a list of discovered devices with metadata (name, path, capabilities).
- Add unit tests for discovery logic (capability filtering, name matching).
**Complete when:**
- `discover_devices()` returns structured device info on any Linux host (may find no Deck devices on dev machine, but should not crash).
- On Deck (Loop 2), correctly identifies stick/button input devices.
- Actual evdev codes documented for use in Step 3 mapping config.

### Step 3 — Define mapping config and types
- [x] Done (2026-03-16)
**Actions:**
- Create `configs/mapping/xbox.toml` with:
  - Evdev-to-Xbox axis mappings (evdev code → HID axis, with invert/scale flags).
  - Evdev-to-Xbox button mappings (evdev code → HID button bit).
  - Deadzone configuration per axis (inner deadzone radius, default ~4000 for sticks).
  - Axis normalization ranges (evdev range → HID range).
- Add mapping config types in `crates/common/` (or `crates/input/`):
  - `MappingConfig` struct parsed from TOML.
  - Axis mapping entry: `{ evdev_code, hid_axis, invert, deadzone }`.
  - Button mapping entry: `{ evdev_code, hid_button }`.
- Add validation: unknown axes/buttons produce clear errors.
**Complete when:**
- `configs/mapping/xbox.toml` exists with all MVP controls mapped.
- Config loads and validates via `serde` + `toml`.
- Unit tests confirm parsing of valid and invalid configs.
- Verified through Loop 1 only.

### Step 4 — Implement input event reading and state tracking
- [x] Done (2026-03-16)
**Actions:**
- Implement `InputReader` that:
  - Opens discovered evdev devices (grab exclusively if needed to avoid conflicts).
  - Reads events in a non-blocking or threaded manner.
  - Maintains current `InputState` (all axis values + button pressed/released).
- Apply mapping from evdev codes to Xbox HID fields using `MappingConfig`.
- Apply deadzone processing: zero out axis values within inner deadzone, scale remaining range.
- Apply axis normalization: convert evdev ranges (e.g., 0–65535 or -32768–32767) to HID report ranges (sticks: -32768–32767 with 0x8000 offset; triggers: 0–1023).
- Provide `current_report() -> InputReport` method that returns the latest mapped state as an `InputReport`.
- Ignored controls (trackpads, rear buttons, gyro) must not affect `InputReport` — filter by mapping config (unmapped codes are dropped).
**Complete when:**
- `InputReader` compiles and can be instantiated with a `MappingConfig`.
- On Deck (Loop 2), reading events from physical controls produces correct `InputReport` values.
- Unit tests verify deadzone math, axis normalization, and button mapping logic.

### Step 5 — Implement `controllerosctl input list` and `controllerosctl input monitor`
- [x] Done (2026-03-16)
**Actions:**
- Add `input list` subcommand to `controllerosctl`:
  - Calls `discover_devices()` from `crates/input/`.
  - Prints each device: path, name, capabilities summary, whether it's selected for mapping.
- Add `input monitor` subcommand:
  - Opens selected devices with mapping config.
  - Prints mapped state changes in real time (button press/release, axis value changes).
  - Use a human-readable format: `A: pressed`, `LX: 12345`, etc.
  - Exit on Ctrl+C.
- Add `crates/input/` as a dependency of `crates/controllerosctl/`.
**Complete when:**
- `controllerosctl input list` prints discovered devices on any Linux host.
- On Deck (Loop 2), `controllerosctl input list` shows Deck input devices with selection indicators.
- On Deck, `controllerosctl input monitor` prints button/axis transitions when controls are physically operated.

### Step 6 — Integrate real input into `hidd` report loop and remove UHID from production path
- [ ] Not started
**Actions:**
- Add `crates/input/` as a dependency of `crates/hidd/`.
- Modify `hidd` to accept a `--mapping-config` argument (path to `configs/mapping/xbox.toml`).
- Replace `PatternState` usage in the main report loop:
  - When `--mapping-config` is provided: create `InputReader`, spawn evdev reading thread, call `current_report()` each tick.
  - When `--mapping-config` is NOT provided (or `--self-test` mode): retain `PatternState` for synthetic test patterns.
- Remove UHID from the production report loop:
  - UHID device creation and report writing should only happen in `--self-test` mode.
  - Normal BLE operation uses only GATT HOG `publish_input_report()`.
- Ensure `--self-test` mode still works as before (UHID + synthetic pattern).
- Update `configs/hid/hid.toml` or add a field pointing to the mapping config path, or keep it as a separate CLI arg.
**Complete when:**
- `hidd` with `--mapping-config` reads real Deck inputs and publishes them via BLE GATT HOG.
- `hidd --self-test` still works with UHID + synthetic pattern (no regression).
- UHID is not used during normal BLE operation.
- Verified on Deck via Loop 2: physical stick/button inputs appear on connected host.

### Step 7 — Documentation
- [ ] Not started
**Actions:**
- Create `docs/mapping.md`:
  - Exact mapping table: Steam Deck evdev code → Xbox HID field.
  - Deadzone and normalization parameters.
  - Which controls are ignored and why.
- Create `docs/input_devices.md`:
  - How evdev devices are discovered and identified on the Deck.
  - Device names, paths, and capabilities expected.
  - How to troubleshoot if discovery fails.
- Update `README.md` with new commands:
  - `controllerosctl input list`
  - `controllerosctl input monitor`
  - `hidd --mapping-config` usage.
**Complete when:**
- Both docs exist with accurate, complete content.
- README reflects new CLI commands and usage.

### Step 8 — Buildroot image integration and end-to-end validation
- [ ] Not started
**Actions:**
- Update `br2-external/board/controlleros/post-build.sh` to install:
  - `configs/mapping/xbox.toml` → `$(TARGET_DIR)/etc/controlleros/mapping/xbox.toml`.
- Update `configs/init/S45hidd` init script to pass `--mapping-config /etc/controlleros/mapping/xbox.toml` to `hidd`.
- Full ISO rebuild + boot on Deck (kernel changes from Step 0 included if not already rebuilt).
- End-to-end validation:
  - Boot Deck, auto-start `hidd` with real input.
  - Pair with Linux host and Android device.
  - Verify all MVP controls work on host (sticks, d-pad, ABXY, LB/RB, LT/RT, Start/Back).
  - Verify ignored controls (trackpads, rear buttons) produce no host-side changes.
  - Run for 10+ minutes: no crashes, stable responsiveness, measure CPU usage.
**Complete when:**
- ISO boots and `hidd` starts with real input mapping automatically.
- Host sees working Xbox-style controller with all MVP controls functional.
- 10-minute stability test passes.

---

## Acceptance Criteria Mapping
- **Prerequisite** (kernel input drivers): Step 0
- A. Device Discovery (`controllerosctl input list`): Steps 0, 2, 5
- B. Mapping Sanity (`controllerosctl input monitor`): Steps 3, 4, 5
- C. Host Controller Test (sticks/buttons/triggers work on host): Steps 4, 6, 8
- D. Ignored Inputs Regression (trackpads/rear buttons produce no changes): Steps 3, 4, 6, 8
- E. Performance (10-minute stability, CPU measurement): Step 8

## Progress Updates
Update this plan by marking steps as **Done** when complete and recording any deviations.
