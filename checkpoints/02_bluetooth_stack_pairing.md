# Checkpoint 02 — Bluetooth Stack + Pairing + Persistence

## Goal
Enable Bluetooth in ControllerOS so a host device can discover and pair with the Steam Deck. Pairing must persist across reboots.

## Scope (MVP)
- BlueZ runs on boot.
- Pairing can be done via CLI tools (minimal UI).
- No HID device exposure yet.

## Non-Goals
- HID descriptor
- Controller mapping
- Automatic reconnection polish (basic reconnect acceptable)

---

## Required Repo Artifacts
- BlueZ config:
  - `configs/bluez/main.conf`
  - `configs/bluez/input.conf`
- Startup:
  - `configs/init/` (or systemd units) ensuring `bluetoothd` starts on boot
- Scripts:
  - `scripts/bt_pairing_mode.sh` (turn on discoverable+pairable)
  - `scripts/bt_show_status.sh` (prints adapter + paired devices)
- Docs:
  - `docs/pairing.md` (exact steps, host-side expectations)
  - `docs/storage.md` (where `/var/lib/bluetooth` is stored/persisted)

---

## Implementation Requirements
1. `bluetoothd` starts on boot.
2. Adapter is powered on and available.
3. Pairing mode script enables discoverable + pairable.
4. Bonds persist:
   - `/var/lib/bluetooth` must be on persistent storage (not tmpfs).

---

## Testable Acceptance Criteria
### A. Service Health
- On Deck:
  - `bluetoothctl show`
- Successful if:
  - `Powered: yes`

### B. Pairing Mode
- Run:
  - `./scripts/bt_pairing_mode.sh`
- Successful if:
  - `Discoverable: yes`
  - `Pairable: yes`

### C. Host Pairing (no host software)
- Host discovers and pairs with `SteamDeck-ControllerOS`.
- Successful if:
  - `bluetoothctl paired-devices` lists host device.

### D. Persistence
- Reboot ControllerOS.
- Successful if:
  - `bluetoothctl paired-devices` still lists the host device.

---

## Definition of Done
- Host can pair with the Deck under ControllerOS.
- Pairing persists across reboot.
- Docs/scripts allow repeating this reliably.
