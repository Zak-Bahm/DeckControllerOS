# Project: Steam Deck ControllerOS (MVP)

## One-sentence summary
Build a minimal, reproducible Linux OS image for Steam Deck that boots into “controller mode” and exposes the Deck as a **Bluetooth HID gamepad** with an **Xbox-style layout** (layout only; not guaranteed XInput branding) without requiring any software installation on the host device.

---

## Desired end state (MVP)
A Git repository that can be built to produce a bootable image (ISO or bootable USB image) that:

1. Boots on a stock Steam Deck.
2. Enables Bluetooth pairing and persists bonds.
3. Exposes a **single** Bluetooth HID gamepad device via BlueZ + UHID/HOGP.
4. Maps a subset of Steam Deck physical controls to an Xbox-style gamepad layout:
   - Sticks, D-pad, ABXY, LB/RB, LT/RT, Start/Back.
5. Ignores non-standard controls in MVP:
   - Trackpads, rear buttons, gyro, touchscreen, haptics.
6. Can be installed alongside SteamOS (dual boot) via documented install steps.

---

## Explicit constraints
- **No USB controller emulation**: the Deck should not be expected to act as a USB HID device.
- **No host software installs**: the host should only need standard Bluetooth support.
- **Single HID interface** only (no composite mouse/keyboard in MVP).
- **Rust-first**: all project code should be written in Rust whenever possible.
- Prefer deterministic builds: pinned versions, reproducible outputs.
- Avoid depending on SteamOS internals for core functionality.
- Avoid “nice-to-have” UI: minimal is acceptable for MVP.

---

## High-level architecture
ControllerOS runs three core services:

1. **Bluetooth service** (BlueZ `bluetoothd`)
2. **HID gamepad service** (Rust daemon using `/dev/uhid` to register HID device + send reports)
3. **Input reader** (Rust module reading Steam Deck input events from `/dev/input/event*` and mapping to HID reports)

Data flow:
`evdev input events → Rust mapping layer → HID report bytes → /dev/uhid → BlueZ → Bluetooth HID (HOGP) → Host OS sees a gamepad`

---

## “Xbox-style layout” definition (MVP)
We emulate a standard HID gamepad whose controls correspond to the conventional Xbox mapping:

- Axes:
  - Left stick: LX, LY
  - Right stick: RX, RY
  - Triggers: LT, RT (analog preferred; digital acceptable for MVP if analog is hard)
- Buttons:
  - A, B, X, Y
  - LB, RB
  - Back (View), Start (Menu)
  - D-pad (hat switch or 4 buttons)

Non-goals in MVP:
- Guide button (often tricky across stacks)
- Vibration/haptics
- Touchpads/gyro/rear buttons

Note: On some hosts, the device may appear as “Generic Bluetooth Gamepad” even if the layout matches Xbox.

---

## Repository layout (expected)
The repo should be structured so an AI tool can begin work immediately:

- `project.md` (this file)
- `instructions.md` (AI execution guardrails)
- `checkpoints/` (the 5 checkpoint files)
- `docs/` (installation, pairing, mapping, storage, boot)
- `scripts/` (build, release, helper scripts)
- `src/` (Rust code; daemons + shared crates)
- `configs/` (BlueZ configs, boot configs, Buildroot configs)
- `out/` (build artifacts; gitignored)

---

## Technology decisions (MVP defaults)
- Build system: **Buildroot** (preferred for minimal, reproducible images).
- Init system: BusyBox init or systemd; choose whichever minimizes friction with BlueZ.
- Bluetooth stack: **BlueZ**.
- HID device creation: Linux **UHID** via `/dev/uhid` from Rust.
- Input reading: Linux **evdev** (read `/dev/input/event*`).

---

## Kernel capabilities required (MVP)
Kernel config must include at minimum:
- Bluetooth support (BT + HCI USB if needed)
- UHID support
- evdev input

---

## Security & safety
- Root access in MVP is acceptable.
- Keep attack surface minimal: no SSH by default unless explicitly required for debugging.
- Document any services that listen on network interfaces.

---

## Success criteria (MVP)
A user can:
1. Install ControllerOS alongside SteamOS on a Steam Deck.
2. Boot into ControllerOS.
3. Pair the Deck with a host over Bluetooth.
4. The host sees a game controller.
5. The Deck’s sticks/buttons operate the host’s controller test UI / a game.

---

## How checkpoints relate to this file
This file provides global context and constraints.
Each `checkpoints/*.md` file defines a concrete, testable milestone with acceptance criteria and required repo artifacts.
