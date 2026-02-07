# Instructions for AI tool execution (Codex-oriented)

## Primary objective
Work strictly through the checkpoint files in `checkpoints/` in order (01 → 05), implementing only what is required to satisfy each checkpoint’s acceptance criteria.

---

## Non-negotiable rules
1. **No scope creep**
   - Do not add features outside the current checkpoint’s requirements.
   - Do not implement touchpads, rear buttons, gyro, haptics, composite HID interfaces, or fancy UI in MVP.

2. **Rust-first**
   - All project code must be in Rust whenever possible (daemons, tooling, parsing, mapping).
   - Shell scripts are allowed only for build orchestration and simple glue.

3. **Best practices**
   - Rust code must compile with `#![forbid(unsafe_code)]` unless a checkpoint explicitly requires unsafe (UHID ioctls may require minimal FFI; keep unsafe minimal and isolated).
   - Use `clippy` + `rustfmt` and keep warnings clean.
   - Favor small, testable crates and clear separation of concerns.

4. **Deterministic builds**
   - Pin versions/commits for buildroot, kernel, and key dependencies.
   - Avoid “latest” downloads without pinning.

5. **Minimal interfaces**
   - Expose exactly one Bluetooth HID gamepad device in MVP.
   - Do not expose mouse/keyboard HID devices.

6. **No host-side requirements**
   - Do not require any software installation on the host beyond Bluetooth.

7. **Documentation is part of done**
   - Every checkpoint requires docs and scripts. Implement them exactly as specified.

---

## Execution workflow
For each checkpoint file:

1. **Read the checkpoint file fully**
   - Extract: Goal, Required Repo Artifacts, Implementation Requirements, Acceptance Criteria.

2. **Plan**
   - Produce a small TODO list tied 1:1 to acceptance criteria.
   - Identify what files must be created/modified.

3. **Implement**
   - Create only the required artifacts.
   - Keep changes small and checkpoint-scoped.

4. **Self-check**
   - Add or update scripts so the acceptance criteria can be verified.
   - Where hardware testing is required, implement “self-test” commands that print expected outputs and exit non-zero on failure.

5. **Stop**
   - Do not begin the next checkpoint until the current one’s criteria are met.

---

## Repo conventions
- Rust workspace at repo root with `Cargo.toml` (workspace).
- Crates:
  - `crates/inputd` (evdev discovery + input state)
  - `crates/hidd` (UHID HID device + report writer daemon)
  - `crates/controllerosctl` (CLI tool for diagnostics/self-tests)
  - `crates/common` (shared types, mapping, config)
- Use `serde` for config formats (`toml` preferred).
- Use `anyhow` for error handling in binaries; `thiserror` for library errors.
- Use `tracing` for logs; keep default logging minimal but helpful.
- Provide `--self-test` on daemons (required by later checkpoints).

---

## Bluetooth/HID guidance (MVP)
- Use BlueZ `bluetoothd` as the system service.
- Use a Rust daemon to:
  - register a HID device via `/dev/uhid`
  - emit HID reports on a steady cadence
- Pairing flow may be CLI-driven in MVP.
- Persist bonding keys in `/var/lib/bluetooth` on a writable partition.

---

## Coding requirements
- Always include:
  - `README.md` updates when new build/run commands appear
  - Minimal unit tests for parsing/mapping logic where possible
- Avoid unnecessary dependencies.
- Avoid unsafe code; if unavoidable:
  - isolate it in one module
  - comment precisely why it is needed
  - wrap with safe abstractions

---

## What “done” looks like per checkpoint
A checkpoint is done only when:
- All required artifacts exist in the repo
- Scripts build/run without manual steps
- Acceptance criteria can be verified as written

---

## Output discipline
- Do not generate large speculative docs or redesign the architecture.
- Do not refactor working code unless required by current checkpoint.
- Do not add multiple alternative implementations. Pick one and proceed.

---

## Troubleshooting discipline
When blocked:
1. Add minimal logging and a `--self-test` path.
2. Reduce problem size (e.g., “enumerate input devices”, “register UHID device”, “send one report”).
3. Document the smallest reproducible issue in `docs/debug.md`.

---

## Quality gates
Before closing a checkpoint, ensure:
- `cargo fmt` clean
- `cargo clippy --all-targets --all-features` clean
- `cargo test` passes for all workspace crates (where tests exist)
