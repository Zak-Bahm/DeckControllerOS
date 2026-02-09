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
- [ ] Missing checkpoint-03 artifacts:
  - No Rust workspace (`Cargo.toml`) or required crates (`crates/hidd`, `crates/controllerosctl`, `crates/common`).
  - No HID config at `configs/hid/hid.toml`.
  - No HID profile documentation at `docs/hid_profile.md`.
  - No build integration to place Rust binaries in the image.
  - No self-test command `controllerosctl hid self-test`.

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

### Step 7 — Boot-time daemon integration
- [ ] Pending
**Actions:**
- Add init script (or systemd unit) to start `hidd` after Bluetooth stack readiness.
- Wire BlueZ HID profile exposure so the Deck is connectable as a single Bluetooth gamepad device (Xbox-style layout semantics via the HID descriptor already defined in `common`).
- Ensure pairing/connection policy supports trusted hosts reconnecting without manual intervention.
- Ensure only one HID gamepad service instance is launched.
**Complete when:**
- After boot, `hidd` is running and prepared to emit reports for a paired host.
- A paired host can fully connect and keep an active controller session (not just bond).
- Host enumerates the Deck as a game controller and receives changing reports.
- Service behavior is first validated with Loop 2 manual restart, then confirmed after reboot once.

### Step 8 — Checkpoint documentation and reproducible self-check flow
- [ ] Pending
**Actions:**
- Add `docs/hid_profile.md` (required) with descriptor breakdown and report format.
- Add checkpoint-03 runbook doc (e.g., `docs/checkpoint03_selfcheck.md`) covering Deck and host validation commands.
- Update `README.md` with checkpoint-03 build/run/test commands and link to `docs/dev_testing_loops.md`.
**Complete when:**
- Documentation fully describes local self-test, host enumeration, and report-visibility validation.
- Docs explicitly separate Loop 1, Loop 2, and full rebuild triggers.

### Step 9 — Validation and quality gates
- [ ] Pending
**Actions:**
- Run `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test`.
- Run checkpoint self-test command on Deck target environment (Loop 2), then one final post-ISO validation boot.
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
