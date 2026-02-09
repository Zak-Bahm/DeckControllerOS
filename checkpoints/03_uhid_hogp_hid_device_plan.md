# Checkpoint 03 Plan — Bluetooth HID Gamepad Exposure (UHID/HOGP)

## Current Repo Evaluation
- [x] Existing prerequisites from checkpoints 01/02:
  - `configs/kernel/steamdeck_defconfig` already enables `CONFIG_UHID=y`, Bluetooth, and `CONFIG_INPUT_EVDEV=y`.
  - BlueZ + DBus boot integration and pairing/persistence scripts are present.
- [ ] Missing checkpoint-03 artifacts:
  - No Rust workspace (`Cargo.toml`) or required crates (`crates/hidd`, `crates/controllerosctl`, `crates/common`).
  - No HID config at `configs/hid/hid.toml`.
  - No HID profile documentation at `docs/hid_profile.md`.
  - No build integration to place Rust binaries in the image.
  - No self-test command `controllerosctl hid self-test`.

## Plan (incremental)

### Step 1 — Create Rust workspace and crate skeletons
- [ ] Pending
**Actions:**
- Add workspace root `Cargo.toml`.
- Create `crates/common`, `crates/hidd`, `crates/controllerosctl`.
- Enforce `#![forbid(unsafe_code)]` in each crate unless a minimal, isolated UHID module requires unsafe.
**Complete when:**
- `cargo check --workspace` passes locally.
- Workspace structure matches checkpoint artifact requirements.

### Step 2 — Define HID profile and shared report types in `common`
- [ ] Pending
**Actions:**
- Implement shared constants/types for report ID, descriptor bytes, and report struct size.
- Add minimal unit tests validating descriptor/report lengths and packing assumptions.
**Complete when:**
- `crates/common` exposes reusable descriptor/report definitions.
- Unit tests pass for descriptor/report invariants.

### Step 3 — Add HID runtime config
- [ ] Pending
**Actions:**
- Create `configs/hid/hid.toml` with device name, vendor/product IDs, report rate, and pattern controls.
- Add config parsing in Rust using `serde` + `toml`.
**Complete when:**
- Config file exists at required path and loads with validation.
- Invalid config returns non-zero with clear error output.

### Step 4 — Implement `hidd` daemon (UHID registration + report loop)
- [ ] Pending
**Actions:**
- Implement UHID device creation via `/dev/uhid`.
- Register one gamepad HID descriptor and emit reports at a stable cadence from config.
- Implement deterministic synthetic pattern (button toggle and/or axis sweep).
- Add `--self-test` mode to validate `/dev/uhid` access and device create/destroy path.
**Complete when:**
- `hidd` starts, registers a single gamepad device, and emits changing reports.
- `hidd --self-test` exits 0 on success and non-zero on failure.

### Step 5 — Implement `controllerosctl hid self-test`
- [ ] Pending
**Actions:**
- Add `controllerosctl` command path: `hid self-test`.
- Print `UHID OK`, descriptor length, and report length.
- Trigger a short test pattern run through `hidd` or shared self-test path.
**Complete when:**
- `controllerosctl hid self-test` satisfies checkpoint output and exit-code requirements.

### Step 6 — Buildroot image integration for Rust binaries
- [ ] Pending
**Actions:**
- Extend Buildroot config/package integration so `hidd` and `controllerosctl` are built and installed into the image.
- Ensure runtime files (`configs/hid/hid.toml`) are included in rootfs.
**Complete when:**
- `./scripts/build.sh` produces an image containing runnable `hidd` and `controllerosctl`.

### Step 7 — Boot-time daemon integration
- [ ] Pending
**Actions:**
- Add init script (or systemd unit) to start `hidd` after Bluetooth stack readiness.
- Ensure only one HID gamepad service instance is launched.
**Complete when:**
- After boot, `hidd` is running and prepared to emit reports for a paired host.

### Step 8 — Checkpoint documentation and reproducible self-check flow
- [ ] Pending
**Actions:**
- Add `docs/hid_profile.md` (required) with descriptor breakdown and report format.
- Add checkpoint-03 runbook doc (e.g., `docs/checkpoint03_selfcheck.md`) covering Deck and host validation commands.
- Update `README.md` with new build/run/test commands.
**Complete when:**
- Documentation fully describes local self-test, host enumeration, and report-visibility validation.

### Step 9 — Validation and quality gates
- [ ] Pending
**Actions:**
- Run `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test`.
- Run checkpoint self-test command on Deck target environment.
**Complete when:**
- All quality gates pass cleanly.
- Acceptance criteria A/B/C can be followed exactly from repo docs/scripts.

---

## Acceptance Criteria Mapping
- A. Local Self-Test (`controllerosctl hid self-test`): Steps 2, 4, 5, 8, 9
- B. Host Enumeration (controller appears on host): Steps 4, 6, 7, 8
- C. Report Visibility (changing input observed): Steps 3, 4, 5, 7, 8

## Progress Updates
Update this plan by marking steps as **Done** when complete and recording any deviations.
