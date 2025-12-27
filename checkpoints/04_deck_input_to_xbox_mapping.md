# Checkpoint 04 — Map Steam Deck Inputs → Xbox-Style HID

## Goal
Read Steam Deck physical control inputs from evdev and map them into the HID gamepad reports so the Deck functions as a controller on the host.

## Scope (MVP)
Map only:
- Left stick (LX/LY)
- Right stick (RX/RY)
- D-pad
- A/B/X/Y
- LB/RB
- LT/RT (analog preferred; digital fallback acceptable if documented)
- Start/Back

Ignore:
- Trackpads
- Rear buttons
- Gyro
- Touchscreen
- Haptics

---

## Required Repo Artifacts
- Rust crates:
  - `crates/inputd/` (evdev discovery + event processing)
  - `crates/common/` (mapping + normalization)
  - `crates/hidd/` updated to accept real input state
- Config:
  - `configs/mapping/xbox.toml` (deadzone, axis scaling, code mapping)
- Docs:
  - `docs/mapping.md` (exact mapping table)
  - `docs/input_devices.md` (how devices are discovered/identified)
- CLI:
  - `controllerosctl input list` (list detected input devices)
  - `controllerosctl input monitor` (prints mapped state changes)

---

## Implementation Requirements
1. Input discovery must be robust:
   - Identify correct Deck input devices without hardcoding `/dev/input/eventX`.
2. Mapping must normalize axes and apply deadzones.
3. Ignored controls must not affect HID state.
4. `hidd` emits reports continuously based on real input state.

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

### C. Host Controller Test
- Host sees the controller and:
  - ABXY, D-pad, sticks, LB/RB, Start/Back work
  - triggers work (analog or documented digital fallback)

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
