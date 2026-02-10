# Checkpoint 03 Plan — Bluetooth HID Gamepad Exposure (UHID/HOGP)

## Current Repo Evaluation
- [x] Existing prerequisites from checkpoints 01/02:
  - `configs/kernel/steamdeck_defconfig` already enables `CONFIG_UHID=y`, Bluetooth, and `CONFIG_INPUT_EVDEV=y`.
  - BlueZ + DBus boot integration and pairing/persistence scripts are present.
- [x] Development loop tooling available:
  - Host payload staging: `scripts/dev_stage_payload.sh`
  - Host HTTP serving: `scripts/dev_http_serve.sh`
  - Deck live updater (in image): `controlleros-dev-update`, `controlleros-dev-list`
  - Workflow guide: `docs/dev_testing_loops.md`
- [x] Implemented checkpoint-03 artifacts:
  - Rust workspace and required crates exist (`Cargo.toml`, `crates/common`, `crates/hidd`, `crates/controllerosctl`).
  - HID config exists at `configs/hid/hid.toml`.
  - Build integration installs `hidd`, `controllerosctl`, and `/etc/controlleros/hid.toml`.
  - `controllerosctl hid self-test` exists and passes local quality gates.
- [ ] Remaining gaps to close checkpoint 03:
  - No HID profile documentation at `docs/hid_profile.md`.
  - No boot-time `hidd` init integration yet (no `S45hidd` script in `configs/init/`).
  - `hidd` currently exposes a generic gamepad descriptor, not an Xbox One BLE-compatible profile.
  - `bluetoothd` currently starts with `--noplugin=input,hog`; HOGP exposure path needs explicit validation/update for host enumeration goals.

## Development Workflow (applies to every step)
- Loop 1 (host-only, default):
  - Use `cargo check/test/clippy/fmt` for rapid iteration without Deck reboot.
- Loop 2 (live Deck update, default for runtime checks):
  - Build binaries locally, stage with `scripts/dev_stage_payload.sh`, serve with `scripts/dev_http_serve.sh`, apply on Deck with `controlleros-dev-update`.
  - Re-run only daemon/self-test commands on Deck after update.
- Full ISO rebuild + reboot (only when required):
  - Use `./scripts/build.sh` only for Buildroot/kernel/init/post-build integration changes.

## Plan (incremental)

### Step 1 — Create Rust workspace and crate skeletons
- [x] Done (2026-02-09)
**Actions:**
- Add workspace root `Cargo.toml`.
- Create `crates/common`, `crates/hidd`, `crates/controllerosctl`.
- Enforce `#![forbid(unsafe_code)]` in each crate unless a minimal, isolated UHID module requires unsafe.
**Complete when:**
- `cargo check --workspace` passes locally.
- Workspace structure matches checkpoint artifact requirements.
- Verified through Loop 1 only (no ISO rebuild required).

### Step 2 — Define HID profile and shared report types in `common`
- [x] Done (2026-02-09)
**Actions:**
- Implement shared constants/types for report ID, descriptor bytes, and report struct size.
- Add minimal unit tests validating descriptor/report lengths and packing assumptions.
**Complete when:**
- `crates/common` exposes reusable descriptor/report definitions.
- Unit tests pass for descriptor/report invariants.
- Verified through Loop 1 only.

### Step 3 — Add HID runtime config
- [x] Done (2026-02-09)
**Actions:**
- Create `configs/hid/hid.toml` with device name, vendor/product IDs, report rate, and pattern controls.
- Add config parsing in Rust using `serde` + `toml`.
**Complete when:**
- Config file exists at required path and loads with validation.
- Invalid config returns non-zero with clear error output.
- Config can be updated on Deck through Loop 2 payload updates.

### Step 4 — Implement `hidd` daemon (UHID registration + report loop)
- [x] Done (2026-02-09)
**Actions:**
- Implement UHID device creation via `/dev/uhid`.
- Register one gamepad HID descriptor and emit reports at a stable cadence from config.
- Implement deterministic synthetic pattern (button toggle and/or axis sweep).
- Add `--self-test` mode to validate `/dev/uhid` access and device create/destroy path.
**Complete when:**
- `hidd` starts, registers a single gamepad device, and emits changing reports.
- `hidd --self-test` exits 0 on success and non-zero on failure.
- Runtime iteration is validated through Loop 2 without reboot.

### Step 5 — Implement `controllerosctl hid self-test`
- [x] Done (2026-02-09)
**Actions:**
- Add `controllerosctl` command path: `hid self-test`.
- Print `UHID OK`, descriptor length, and report length.
- Trigger a short test pattern run through `hidd` or shared self-test path.
**Complete when:**
- `controllerosctl hid self-test` satisfies checkpoint output and exit-code requirements.
- Command is exercised on Deck using Loop 2 updates before image integration.

### Step 6 — Buildroot image integration for Rust binaries
- [x] Done (2026-02-09)
**Actions:**
- Extend Buildroot config/package integration so `hidd` and `controllerosctl` are built and installed into the image.
- Ensure runtime files (`configs/hid/hid.toml`) are included in rootfs.
**Complete when:**
- `./scripts/build.sh` produces an image containing runnable `hidd` and `controllerosctl`.
- This is the first step that requires the full rebuild/reboot loop.

### Step 7 — Update `hidd` to Xbox One BLE-compatible HID profile
- [ ] Pending
**Actions:**
- Replace the current generic descriptor/report packing with an Xbox One BLE-compatible profile (default target: model 1708 compatibility).
- Add shared report types/constants for Xbox-style report layout and report IDs while keeping a single HID interface.
- Extend HID config schema with profile identity fields (VID/PID/version/profile mode), defaulting to Microsoft Xbox-compatible values.
- Update UHID create payload fields to use configured version/identity values (remove hardcoded version assumptions).
- Add minimal output-report handling in `hidd` (drain/log/drop safely; no haptics implementation in MVP).
- Add `docs/hid_profile.md` documenting descriptor, report format, and known host behavior.
- Extend `controllerosctl hid self-test` output with active profile + identity in addition to descriptor/report lengths.
**Complete when:**
- `controllerosctl hid self-test` prints Xbox profile identity and exits 0.
- A paired host identifies the controller as Xbox-compatible (or platform-equivalent XInput-class BLE gamepad naming).
- Host receives changing test-pattern input while using the Xbox profile.

### Step 8 — Boot-time daemon integration and reproducible self-check flow
- [ ] Pending
**Actions:**
- Add init script (or systemd unit) to start `hidd` after Bluetooth stack readiness.
- Update Bluetooth daemon startup/config as needed so HID over GATT is exposed correctly during runtime.
- Create script to run on host that tests current state end-to-end: pairing, connecting, receiving input test pattern.
**Complete when:**
- After boot, `hidd` is running and prepared to emit reports for a paired host.
- A paired host can fully connect and keep an active controller session (not just bond).
- Host enumerates the Deck as a game controller and receives changing reports.
- Service behavior is first validated with Loop 2 manual restart, then confirmed with host script.

---

## Acceptance Criteria Mapping
- A. Local Self-Test (`controllerosctl hid self-test`): Steps 2, 4, 5, 7
- B. Host Enumeration (controller appears on host): Steps 4, 6, 7, 8
- C. Report Visibility (changing input observed): Steps 3, 4, 5, 7, 8

## Progress Updates
Update this plan by marking steps as **Done** when complete and recording any deviations.
