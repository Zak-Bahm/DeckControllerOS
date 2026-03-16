# Checkpoint 04 — Map Steam Deck Inputs → Xbox-Style HID

## Goal
Read Steam Deck physical control inputs via hidraw and map them into the HID gamepad reports so the Deck functions as a controller on the host.

## Scope (MVP)
Map only:
- Left stick (LX/LY)
- Right stick (RX/RY)
- D-pad
- A/B/X/Y
- LB/RB
- LT/RT (analog)
- Start/Back
- Home (Steam button)
- LS/RS (stick clicks)

Ignore:
- Trackpads
- Rear buttons (grips)
- Gyro
- Touchscreen
- Haptics

---

## Input Method: hidraw

The Steam Deck controller is read via the kernel's **hidraw** interface, not evdev. The `hid-steam` driver creates a virtual client HID device (`client_hdev`) that forwards raw 64-byte reports when opened. This is the same mechanism Steam uses to read the controller.

### Why hidraw instead of evdev
The `hid-steam` driver has two guards that prevent evdev from working on the Deck:
1. `steam_input_open()` skips `steam_set_lizard_mode(false)` for devices with `STEAM_QUIRK_DECK`
2. `steam_do_deck_input_event()` checks a `gamepad_mode` flag (defaults to false) and returns early

While guard #1 can be patched, guard #2 requires holding the options button to toggle `gamepad_mode`. The hidraw approach bypasses both guards entirely by reading raw HID data directly from the controller.

### How it works
1. **Discovery**: Scan `/sys/class/hidraw/`, match VID `0x28DE` PID `0x1205`, select the device without an `input/` subdirectory (the `client_hdev`, typically `hidraw2`)
2. **Open**: Opening the client hidraw triggers `steam_client_ll_open()` in the kernel, which sets `client_opened = true` and unregisters evdev input devices
3. **Disable lizard mode**: Send two HID feature reports via `HIDIOCSFEATURE` ioctl:
   - `ID_CLEAR_DIGITAL_MAPPINGS` (0x81) — stops keyboard/mouse emulation
   - `ID_SET_SETTINGS_VALUES` (0x87) — disables trackpad modes, sets max click pressure, disables watchdog
4. **Read**: Raw 64-byte reports with type `0x09` (`ID_CONTROLLER_DECK_STATE`) contain button bits and axis values at fixed byte offsets
5. **Parse**: Extract buttons from data[8-14] bit fields, axes from data[44-55] as little-endian i16 values

### Kernel requirements
- `CONFIG_HIDRAW=y` in kernel defconfig (creates `/sys/class/hidraw/` and `/dev/hidraw*` nodes)
- `CONFIG_HID_STEAM=y` (Steam Deck controller driver)

### Raw report format (type 0x09, 64 bytes)

**Buttons (bit positions):**
| Byte | Bit | Button |
|------|-----|--------|
| data[8] | 7 | A |
| data[8] | 6 | X |
| data[8] | 5 | B |
| data[8] | 4 | Y |
| data[8] | 3 | LB |
| data[8] | 2 | RB |
| data[9] | 0 | D-pad Up |
| data[9] | 1 | D-pad Right |
| data[9] | 2 | D-pad Left |
| data[9] | 3 | D-pad Down |
| data[9] | 4 | Select/Back |
| data[9] | 5 | Home/Steam |
| data[9] | 6 | Start |
| data[10] | 6 | LS (left stick click) |
| data[11] | 2 | RS (right stick click) |

**Axes (little-endian i16):**
| Offset | Axis | Range | Notes |
|--------|------|-------|-------|
| data[48..50] | Left stick X | -32768..32767 | |
| data[50..52] | Left stick Y | -32768..32767 | Raw Y-up positive; negate for standard |
| data[52..54] | Right stick X | -32768..32767 | |
| data[54..56] | Right stick Y | -32768..32767 | Raw Y-up positive; negate for standard |
| data[44..46] | Left trigger | 0..32767 | |
| data[46..48] | Right trigger | 0..32767 | |

---

## Required Repo Artifacts
- Rust crates:
  - `crates/input/` — **library crate** for hidraw discovery, lizard mode control, raw report parsing, and normalization (imported by `hidd` and `controllerosctl`)
  - `crates/common/` (InputReport struct, HID constants)
  - `crates/hidd/` updated to accept real input state from `crates/input/` and replace synthetic pattern generation
- Config:
  - `configs/mapping/xbox.toml` (deadzone, axis scaling parameters)
  - `configs/kernel/steamdeck_defconfig` with `CONFIG_HIDRAW=y`
- Docs:
  - `docs/mapping.md` (exact mapping table)
  - `docs/input_devices.md` (how devices are discovered/identified)
- CLI:
  - `controllerosctl input list` (list detected evdev input devices)
  - `controllerosctl input monitor` (prints mapped state changes from hidraw)

**Architecture note:** `hidd` owns the main report loop, the BLE GATT HOG connection, and report transmission. The hidraw input reader runs as a thread within `hidd`, feeding `InputReport` structs into the existing report loop. This avoids unnecessary IPC between separate daemons. The `crates/input/` library provides hidraw discovery, lizard mode control, raw report parsing, and mapping logic that `hidd` calls directly.

---

## Implementation Requirements
1. Input discovery must be robust:
   - Identify correct Deck hidraw device without hardcoding `/dev/hidraw*` numbers.
   - Select the client device (no `input/` subdirectory in sysfs) among Valve VID devices.
2. Lizard mode must be disabled via HID feature reports before reading input.
3. Mapping must normalize axes and apply deadzones.
4. Ignored controls must not affect HID state.
5. `hidd` emits reports continuously based on real input state (replacing synthetic pattern generation).
6. Remove UHID from the production report loop — UHID should only be used in `--self-test` mode, not during normal BLE operation. The BLE data path is via GATT HOG, not UHID.

---

## Testable Acceptance Criteria
### A. Device Discovery
- Run:
  - `controllerosctl input list`
- Successful if:
  - prints at least one device used for sticks/buttons and clearly indicates selection.

### B. Mapping Sanity
- Run:
  - `controllerosctl input monitor`
- Successful if:
  - pressing ABXY prints transitions
  - moving sticks prints axis changes
  - triggers show analog values 0..1023
  - d-pad shows hat directions (N/S/E/W/NE/SE/SW/NW)

### C. Host Controller Test
- Host sees the controller and:
  - ABXY, D-pad, sticks, LB/RB, LS/RS, Start/Back, Home work
  - triggers work (analog)

### D. Ignored Inputs Regression
- Touch trackpads / rear buttons:
  - Host must not show changes (or changes are explicitly documented and fixed).

### E. Performance
- Running for 10 minutes:
  - no crashes
  - stable responsiveness
  - CPU usage reasonable (document measurement)

---

## Definition of Done
- You can use the Steam Deck as a Bluetooth controller on a host with the MVP Xbox-style layout.
