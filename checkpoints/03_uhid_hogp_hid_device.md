# Checkpoint 03 — Bluetooth HID Gamepad Exposure (UHID/HOGP)

## Goal
Expose a **single** Bluetooth HID gamepad device to the paired host using BlueZ + UHID. The device must be recognized by the host as a controller and must receive changing input reports (test pattern ok).

## Scope (MVP)
- Implement one HID gamepad profile (Xbox-style layout semantics).
- Reports can be synthetic (test pattern) for this checkpoint.
- No Steam Deck input mapping required yet.

## Non-Goals
- Multiple HID interfaces
- Mouse/keyboard emulation
- Trackpads/rear buttons/gyro

---

## Required Repo Artifacts
- Rust workspace with crates:
  - `crates/hidd/` (UHID + report sender daemon)
  - `crates/controllerosctl/` (CLI utility for diagnostics/self-tests)
  - `crates/common/` (shared HID types/structs)
- HID docs:
  - `docs/hid_profile.md` (descriptor + report format explanation)
- Config:
  - `configs/hid/hid.toml` (report rate, device name, etc.)
- Daemon integration:
  - init/systemd unit to start `hidd` (optional for MVP, but recommended)

---

## Implementation Requirements
1. Kernel provides `/dev/uhid`.
2. `hidd` registers a HID gamepad device via UHID and emits reports at a stable rate.
3. Host enumerates the device as a controller.
4. `controllerosctl hid self-test`:
   - prints descriptor/report size
   - sends a short test pattern (A button toggle or axis sweep)
   - exits 0 on success, non-zero on failure.

---

## Testable Acceptance Criteria
### A. Local Self-Test
- Run on Deck:
  - `controllerosctl hid self-test`
- Successful if:
  - prints “UHID OK”
  - prints descriptor length + report length
  - exits 0

### B. Host Enumeration
- Host pairs and connects.
- Successful if:
  - Windows: controller appears in `joy.cpl`, OR
  - Linux: new `/dev/input/js*` exists, OR
  - macOS: controller visible in system game controller list (if available).

### C. Report Visibility
- While test pattern runs:
  - Host controller test UI shows a changing input (button or axis).

---

## Definition of Done
- The host sees a gamepad and receives changing input reports over Bluetooth.
