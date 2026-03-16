# Checkpoint 03 Plan — Bluetooth HID Gamepad Exposure (UHID/HOGP)

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
- [x] Done (2026-03-15, validated on Linux host + Android)

**Architecture note:** The BLE HID path uses a custom GATT HID-over-GATT Profile (HOGP) application registered with BlueZ via D-Bus (`RegisterApplication` + `RegisterAdvertisement`), implemented in `crates/hidd/src/hog.rs`. UHID is retained for `--self-test` diagnostics only and is not part of the BLE data path. The daemon sends reports to both UHID (local) and GATT HOG (BLE) during the pattern loop; the UHID path should be removed from the production report loop in checkpoint 04.

**Actions:**
- Replace the current generic descriptor/report packing with an Xbox One BLE-compatible profile (default target: model 1708 compatibility).
- Add shared report types/constants for Xbox-style report layout and report IDs while keeping a single HID interface.
- Extend HID config schema with profile identity fields (VID/PID/version/profile mode), defaulting to Microsoft Xbox-compatible values.
- Update UHID create payload fields to use configured version/identity values (remove hardcoded version assumptions).
- Add minimal output-report handling in `hidd` (drain/log/drop safely; no haptics implementation in MVP).
- Add `docs/hid_profile.md` documenting descriptor, report format, and known host behavior.
- Extend `controllerosctl hid self-test` output with active profile + identity in addition to descriptor/report lengths.
- Implement BLE GATT HOGP runtime (`hog.rs`): HID Service (0x1812) with report characteristics and Report Reference descriptors for all 4 report IDs, Battery Service (0x180F), Device Information Service (0x180A) with PnP ID, and LE advertisement with gamepad appearance.
**Complete when:**
- `controllerosctl hid self-test` prints Xbox profile identity and exits 0.
- A paired host identifies the controller as Xbox-compatible (or platform-equivalent XInput-class BLE gamepad naming).
- Host receives changing test-pattern input while using the Xbox profile.

### Step 8 — Boot-time daemon integration and reproducible self-check flow
- [x] Done (2026-03-15)
**Actions:**
- Added `configs/init/S45hidd` init script to start `hidd` after Bluetooth stack readiness (waits up to 10s for adapter).
- `hidd` registers a `NoInputNoOutput` BlueZ pairing agent via D-Bus (`org.bluez.Agent1`) for automatic Just Works BLE pairing.
- Created `scripts/bt_checkpoint03_host_validate.sh` for host-side end-to-end testing (discovery, pair, trust, connect, input observation).
- Created `configs/dev/controlleros-dev-debug` for consolidated on-Deck diagnostics.
**Validated:**
- Linux host: discovery, pairing (`Bonded: yes`), connection, GATT service enumeration (GAP, GATT, DIS, Battery, HID), notification subscription, `evtest` confirmed BTN_SOUTH test pattern on `/dev/input/event16`.
- Android: discovery, pairing, connection as gamepad.
- Deck logs confirmed agent authorization, StartNotify for all 3 input reports + battery level.

---

## Acceptance Criteria Mapping
- A. Local Self-Test (`controllerosctl hid self-test`): Steps 2, 4, 5, 7
- B. Host Enumeration (controller appears on host): Steps 4, 6, 7, 8
- C. Report Visibility (changing input observed): Steps 3, 4, 5, 7, 8

## Progress Updates
Update this plan by marking steps as **Done** when complete and recording any deviations.
