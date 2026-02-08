# Checkpoint 02 Plan — Bluetooth Stack + Pairing + Persistence

## Plan (incremental)

### Step 1 — Review current Bluetooth setup and boot integration
- [x] Done
**Actions:**
- Inspect existing BlueZ configs, init/systemd units, and scripts.
- Confirm whether `bluetoothd` is already started on boot.
- Verify DBus is enabled and started (BlueZ dependency).
**Complete when:**
- A short list of missing/needed artifacts is identified for checkpoint 02.
- The exact init mechanism to hook into is confirmed.

### Step 2 — Add BlueZ configuration files
- [x] Done
**Actions:**
- Create `configs/bluez/main.conf`.
- Create `configs/bluez/input.conf`.
**Notes:**
- Keep defaults minimal; discoverable/pairable are set via `bluetoothctl` in this checkpoint.
**Complete when:**
- Both config files exist with minimal, explicit settings.

### Step 3 — Add Bluetooth support to the image (kernel + firmware)
- [x] Done
**Actions:**
- Confirm Steam Deck Bluetooth chipset (USB/PCI IDs).
  - Deck LCD shows USB `13d3:3553` (IMC Networks) with Realtek firmware
    `rtl_bt/rtl8822cu_fw.bin` + `rtl_bt/rtl8822cu_config.bin`.
- Enable required kernel options in `configs/kernel/steamdeck_defconfig`:
  - Core Bluetooth + USB HCI: `CONFIG_BT`, `CONFIG_BT_HCIUSB`, `CONFIG_BT_HCIBTUSB`.
  - Realtek driver: `CONFIG_BT_RTL`.
  - LE security and profiles: `CONFIG_BT_LE`, `CONFIG_BT_SMP`, `CONFIG_BT_RFCOMM`,
    `CONFIG_BT_BNEP`.
  - Support bits: `CONFIG_RFKILL`, `CONFIG_UHID` (for later HID checkpoint).
- Ensure firmware selection covers Realtek 88xx:
  - `BR2_PACKAGE_LINUX_FIRMWARE_RTL_88XX_BT=y` (matches `rtl8822cu_*`).
- Ensure `bluetoothctl` is available in the image.
**Complete when:**
- `/sys/class/bluetooth/hci0` exists on the Deck.
- `dmesg` shows the chipset driver + firmware load without errors.

### Step 4 — Ensure bluetoothd starts on boot
- [x] Done
**Actions:**
- Add init/systemd unit(s) under `configs/init/` (or systemd units) to start `bluetoothd`.
- Ensure adapter is powered on at boot (via service or script).
- Verify `bluetoothd` is using `/etc/bluetooth/main.conf`.
**Complete when:**
- `bluetoothd` starts automatically at boot.
- Adapter powers on without manual commands.

### Step 5 — Add pairing and status scripts
- [ ] Todo
**Actions:**
- Create `scripts/bt_pairing_mode.sh` to set discoverable + pairable.
- Create `scripts/bt_show_status.sh` to show adapter state and paired devices.
**Complete when:**
- Both scripts exist, executable, and use `bluetoothctl` to report/act.
- Running `bt_pairing_mode.sh` results in Discoverable/Pairable yes.

### Step 6 — Ensure bond persistence storage
- [ ] Todo
**Actions:**
- Define a persistent mount (partition/overlay) for `/var/lib/bluetooth`.
- Mount it at boot before `bluetoothd` starts.
**Complete when:**
- `/var/lib/bluetooth` persists across reboot in ControllerOS.

### Step 7 — Documentation for pairing and storage
- [ ] Todo
**Actions:**
- Write `docs/pairing.md` with exact CLI steps and host expectations.
- Write `docs/storage.md` describing where `/var/lib/bluetooth` lives and how it persists.
**Complete when:**
- Docs exist and match scripts/behavior.

### Step 8 — Self-check instructions
- [ ] Todo
**Actions:**
- Add checklist to docs or scripts to validate Powered/Discoverable/Pairable and persistence.
**Complete when:**
- Acceptance criteria steps are fully reproducible from repo content.

---

## Acceptance Criteria Mapping
- Service Health (Powered yes): Steps 3, 4
- Pairing Mode (Discoverable/Pairable yes): Steps 4, 5
- Host Pairing (paired-devices shows host): Steps 4, 5, 6
- Persistence across reboot: Steps 6, 7, 8

## Progress Updates
Update this plan by marking steps as **Done** when complete and recording any deviations.
