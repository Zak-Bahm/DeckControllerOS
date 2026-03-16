# Checkpoint 03.5 — BLE HOGP Interop Stabilization (Post-Step-7)

## Why this checkpoint exists
Step 7 introduced an Xbox One S 1708 BLE profile and a BlueZ GATT HOG runtime in `hidd`.
This checkpoint isolated and resolved known BLE HOGP interop gaps before proceeding to checkpoint 04 input mapping.

## Baseline assumptions
- Checkpoint 03 Step 7 code is present (Xbox descriptor + `crates/hidd/src/hog.rs`).
- Runtime validation done with manually started `hidd` via `controlleros-dev-debug hidd-run`.
- Two real hosts used for validation:
  - Linux host (detailed GATT/input verification via `evtest`).
  - Android device (Bluetooth settings pairing + gamepad recognition).

## Known gaps to fix in this checkpoint

**Status: COMPLETE (2026-03-15). All four issues verified on real hosts (Linux + Android). See validation results below.**

1. BLE report framing mismatch between HOG report characteristics and UHID report bytes.
   - **Fix in code:** `ble_input_payload_from_uhid()` strips the report ID byte before writing to BLE characteristics; Report Reference descriptors identify each report.
2. Xbox identity fields (VID/PID/version) not exposed through BLE Device Information/PnP path.
   - **Fix in code:** Device Information Service (0x180A) with PnP ID characteristic (0x2A50) encoding VID/PID/version from config.
3. HID GATT permissions/security flags not enforcing encrypted bonded access.
   - **Fix in code:** All HID characteristics use `encrypt-read`, `encrypt-write`, and `encrypt-notify` flags.
4. Descriptor-declared report IDs (`0x01/0x02/0x03/0x04`) not fully represented in exposed GATT report characteristics.
   - **Fix in code:** All 4 report IDs have dedicated characteristics with Report Reference descriptors (0x2908).

## Scope
- Fix only BLE HOGP interoperability issues listed above.
- Keep a single HID gamepad interface.
- Keep synthetic pattern generation (no Deck evdev mapping changes in this checkpoint).
- Keep haptics unimplemented (output reports may be parsed/logged/dropped).

## Non-goals
- Boot-time `hidd` service integration (`S45hidd`/systemd) and host automation script creation from Step 8.
- Input mapping from Steam Deck controls (checkpoint 04).
- Multi-interface/composite HID.

## Files expected to be modified
- `crates/hidd/src/main.rs`
- `crates/hidd/src/hog.rs`
- `crates/common/src/hid.rs` (only if helper APIs/constants are needed)
- `scripts/bt_pairing_mode.sh`
- `docs/hid_profile.md`
- `README.md` (only commands/validation notes that change in this checkpoint)

## Implementation requirements

### 1) Fix BLE report framing
Requirement:
- HOG input characteristic values must not include an in-band report ID byte when Report Reference already identifies the report.
- UHID input path remains unchanged (still uses report ID in UHID report payload).

Implementation notes:
- Add a clear boundary in code between:
  - UHID report bytes (with ID),
  - BLE report characteristic values (payload only).
- Ensure output-report parsing accepts BLE payloads and normalizes them for existing parser/logging.
- Add explicit logs when host enables/disables notifications (`StartNotify`/`StopNotify`).

### 2) Expose BLE identity (DIS + PnP)
Requirement:
- BLE side must expose identity derived from config profile fields:
  - `profile.vendor_id`
  - `profile.product_id`
  - `profile.version`
- Add Device Information Service (`0x180A`) with PnP ID characteristic (`0x2A50`).

Implementation notes:
- Use VID source `0x02` (USB) for PnP ID encoding.
- Preserve existing UHID identity behavior.
- Document byte layout in `docs/hid_profile.md`.

### 3) Enforce HID security flags + deterministic pairing agent setup
Requirement:
- HID sensitive characteristics/descriptors must require encrypted access.
- Pairing flow should initialize a deterministic agent capability to reduce host-dependent failures.

Implementation notes:
- Apply appropriate BlueZ server flags:
  - `encrypt-read`
  - `encrypt-write`
  - `encrypt-notify`
- Update pairing helper script to set:
  - `agent NoInputNoOutput`
  - `default-agent`
- Do not introduce interactive/manual pairing dependencies.

### 4) Align exposed GATT reports with descriptor report IDs
Requirement:
- If descriptor advertises report IDs `0x01`, `0x02`, `0x03`, `0x04`, exposed report characteristics and report-reference descriptors must be consistent with that topology.

Implementation notes:
- Represent all required report IDs through report characteristics + `0x2908` Report Reference descriptors.
- Dynamic test pattern may remain on report `0x01`; additional input reports may be static placeholders.
- Keep one HID service/interface only.

## Real-life validation protocol (required after each issue fix)

### Pre-flight reset (before each validation run)
On Deck:
```sh
bluetoothctl remove <HOST_MAC> || true
```

On host:
```sh
bluetoothctl remove <DECK_MAC> || true
```

Restart runtime on Deck:
```sh
/etc/init.d/S40bluetoothd restart
/var/lib/controlleros/dev/bin/hidd --config /var/lib/controlleros/dev/configs/hid/hid.toml
```

### Linux host validation commands
Pair/connect:
```sh
bluetoothctl
scan on
pair <DECK_MAC>
trust <DECK_MAC>
connect <DECK_MAC>
info <DECK_MAC>
```

Input visibility (example):
```sh
evtest
# or
jstest /dev/input/jsX
```

### Validation required for each issue

Issue 1 (framing):
- Host remains connected for >= 60 seconds.
- Changing synthetic inputs are visible continuously.
- No repeated size/parse errors in `hidd` logs.

Issue 2 (identity):
- Linux GATT inspection confirms PnP ID values match config VID/PID/version.
- Previously failing host shows improved classification (gamepad/Xbox-compatible class where applicable), not only generic unknown BLE peripheral.

Issue 3 (security):
- Unpaired host cannot read protected HID data.
- Paired/bonded host can read and use controller normally.
- `bluetoothctl info <DECK_MAC>` shows `Paired: yes`, `Bonded: yes`, `Connected: yes`.

Issue 4 (report topology):
- GATT report references match descriptor IDs/types.
- No immediate disconnect/profile rejection after connect on either host.
- 5-minute idle + input soak passes on previously failing host.

## Final acceptance criteria for checkpoint 03.5
1. Both validation hosts can discover, pair, trust, and connect.
2. Both hosts receive changing synthetic input for at least 60 seconds.
3. Previously failing host no longer fails in the prior mode (not visible / generic-only / cannot connect).
4. BLE GATT shape is internally consistent:
   - report IDs in descriptor match report references exposed by HOG service.
   - identity fields are exposed over BLE through DIS/PnP and match config.
5. Security behavior is consistent:
   - HID access requires encrypted bonded link where configured.

## Required evidence to collect
- Deck-side `hidd` logs from successful connect/input session.
- Host-side pairing/connect logs for both hosts.
- Linux host GATT dump or readout showing PnP ID and report references.
- Brief note in `docs/hid_profile.md` for any host-specific naming behavior still observed.

## Quality gates (must pass)
```sh
cargo fmt --all
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo test --workspace
```

## Validation results (2026-03-15)

### Hosts tested
1. **Linux host** (Ubuntu, BlueZ, `evtest` for input verification)
2. **Android device** (standard Bluetooth settings, recognized as gamepad)

### Evidence collected
- Deck-side hidd logs: `out/dev-logs/20260315T235056Z_deck_exitunknown.log`
  - Agent authorized device, StartNotify for battery + all 3 input report IDs
- Host-side validation log: `out/host-logs/validate_20260315_165037.log`
  - Discovery OK, Pairing OK (`Bonded: yes`, `Paired: yes`), Connect OK
  - All 5 GATT services resolved (GAP, GATT, DIS, Battery, HID)
  - `Modalias: usb:v045Ep02FDd0408` — correct Xbox One S 1708 PnP ID
  - Battery: 100%
  - `/dev/input/js0` appeared as joystick
- Linux host `evtest` output confirmed BTN_SOUTH (A button) toggling at ~4Hz test pattern on `/dev/input/event16`
- Android paired and connected successfully via Bluetooth settings

### Per-issue results
1. **Framing**: Host remained connected, continuous BTN_SOUTH events, no parse errors in hidd logs.
2. **Identity**: `Modalias: usb:v045Ep02FDd0408` matches config VID/PID/version. Both hosts classified device as gamepad.
3. **Security**: Pairing succeeded with `NoInputNoOutput` agent (Just Works). `Paired: yes`, `Bonded: yes`, `Connected: yes`.
4. **Report topology**: All GATT report references resolved. No disconnect/rejection on either host.

### Additional fix applied during validation
- BLE pairing failed initially (`AuthenticationFailed`) because no BlueZ pairing agent was registered. Fixed by adding `org.bluez.Agent1` D-Bus agent registration directly in `hidd` (`crates/hidd/src/hog.rs`) with `NoInputNoOutput` capability. Agent lifecycle is tied to hidd (registered on startup, unregistered on shutdown).

## Stop condition
Checkpoint 03.5 acceptance criteria met on 2 real hosts. Checkpoint 04 work may proceed.
