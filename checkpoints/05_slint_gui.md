# Checkpoint 05 — Basic On-Screen GUI (Slint)

## Goal
Provide a minimal touchscreen GUI on the Steam Deck's built-in display using the Slint UI framework, giving the user direct control over Bluetooth connections and system actions without needing a keyboard or SSH.

## Scope (MVP)
- Single-screen UI rendered on the framebuffer (no X11/Wayland compositor needed).
- Touch input from the Deck's touchscreen for all interaction.
- Functional, not pretty — layout clarity over visual polish.

---

## Features

### 1. Connected Devices List
- Display all paired/connected Bluetooth devices (name, MAC address, connection status).
- Auto-refresh on state changes or on a short polling interval.

### 2. Device Actions
For each listed device, provide buttons to:
- **Disconnect** — drop the active BLE connection.
- **Reconnect** — re-establish a connection to a paired device.
- **Forget** — remove pairing/bond (equivalent to `bluetoothctl remove <MAC>`).

### 3. System Actions
Global buttons at the bottom of the screen:
- **Restart Stack** — restart BlueZ, `hidd`, and the GUI itself (in order: stop hidd → stop bluetooth → start bluetooth → start hidd → restart GUI).
- **Restart** — reboot the device (`reboot`).
- **Power Off** — shut down the device (`poweroff`).

---

## Required Repo Artifacts
- Rust crates:
  - `crates/gui/` — Slint-based GUI application crate (binary: `controlleros-gui`)
  - `crates/gui/ui/` — Slint `.slint` UI definition files
- Config:
  - `configs/buildroot/controlleros_defconfig` updated to include framebuffer and touchscreen kernel support
- Docs:
  - `docs/gui.md` (architecture, Slint integration, touchscreen setup, framebuffer backend)
- Scripts:
  - `scripts/build.sh` updated to cross-compile and install `controlleros-gui` into the image
- Init:
  - Init script or service to launch `controlleros-gui` on boot (replaces or runs alongside getty on tty1)

**Architecture note:** `controlleros-gui` communicates with BlueZ over D-Bus (using `zbus` or `dbus`) to list paired devices, connect, disconnect, and remove devices. System actions (`reboot`, `poweroff`) are executed via direct command invocation. The GUI does not interact with `hidd` directly — `hidd` continues to run as its own service. The GUI reads Bluetooth state independently from BlueZ.

---

## Implementation Requirements
1. **Slint with framebuffer backend**: Use Slint's `LinuxKMS` or software renderer targeting the Linux framebuffer (`/dev/fb0` or DRM/KMS). No display server required.
2. **Touchscreen input**: Slint must receive touch events from the Deck's touchscreen via libinput or evdev. Verify the touchscreen device is accessible and coordinates map correctly to the display.
3. **BlueZ D-Bus integration**: Query `org.bluez` over the system D-Bus to:
   - Enumerate paired devices (`org.bluez.Device1` objects).
   - Read properties: `Name`, `Address`, `Connected`, `Paired`, `Trusted`.
   - Call `Connect()`, `Disconnect()`, `Remove()` on device objects.
4. **System commands**: `reboot`, `poweroff`, and stack restart are invoked as child processes. Confirm with a dialog before executing. **Restart Stack** must stop services in reverse dependency order (`S45hidd` → `S44bluetooth`), then start them in forward order (`S44bluetooth` → `S45hidd`), and finally re-exec or restart the GUI process itself.
5. **Auto-start**: The GUI must launch automatically on boot. Use an init script (`S50gui` or similar) that starts after BlueZ (`S44bluetooth`) and `hidd` (`S45hidd`).
6. **Kernel config**: Ensure the defconfig includes:
   - `CONFIG_FB=y` or DRM/KMS support for the Deck's display
   - `CONFIG_INPUT_TOUCHSCREEN=y`
   - Appropriate Deck touchscreen driver (likely `CONFIG_TOUCHSCREEN_USB_COMPOSITE` or an i2c touchscreen driver)
7. **Graceful coexistence**: The GUI must not interfere with `hidd` or BlueZ operation. It is a read/control overlay, not a replacement for any existing service.
8. **Use Slint standard widgets**: The UI must `import { ... } from "std-widgets.slint";` and use standard library components wherever applicable. Prefer `Button`, `ListView`, `ScrollView`, `GroupBox`, `VerticalBox`, `HorizontalBox`, `GridBox`, `LineEdit`, `ComboBox`, `Switch`, `ProgressIndicator`, `Spinner`, and `AboutSlint` over custom equivalents. Only create custom components when no suitable `std-widgets.slint` component exists. This ensures consistent theming, accessibility defaults, and reduces maintenance burden.

---

## Testable Acceptance Criteria

### A. GUI Launches on Boot
- Boot the ControllerOS image on a Steam Deck.
- Successful if:
  - The Slint GUI is visible on the Deck's screen within 10 seconds of boot.
  - No manual intervention required.

### B. Device List
- Pair a host device via the existing pairing flow (or use a pre-paired device).
- Successful if:
  - The paired device appears in the GUI's device list with its name and connection status.
  - The list updates when a device connects or disconnects.

### C. Disconnect / Reconnect
- With a connected host device:
  - Tap **Disconnect** in the GUI.
- Successful if:
  - The host loses the controller connection.
  - The GUI shows the device as disconnected.
  - Tap **Reconnect** — the host regains the controller.

### D. Forget Device
- With a paired (disconnected) device:
  - Tap **Forget** in the GUI.
- Successful if:
  - The device is removed from BlueZ's paired list.
  - The device no longer appears in the GUI.
  - The host must re-pair to connect again.

### E. Restart Stack
- With a connected host device, tap **Restart Stack** in the GUI.
- Successful if:
  - A confirmation prompt appears.
  - Confirming stops `hidd` and BlueZ, then restarts them and the GUI.
  - After restart, the GUI reappears, the device list repopulates, and the host can reconnect.

### F. Restart / Power Off
- Tap **Restart** in the GUI.
- Successful if:
  - A confirmation prompt appears.
  - Confirming causes the Deck to reboot.
- Tap **Power Off** in the GUI.
- Successful if:
  - A confirmation prompt appears.
  - Confirming causes the Deck to shut down.

### G. No Regression
- With the GUI running, the existing controller functionality (checkpoint 04) must still work:
  - `hidd` runs and transmits HID reports.
  - Host sees a working controller.

---

## Definition of Done
- The Steam Deck boots into a touchscreen GUI that displays Bluetooth device status and allows the user to manage connections and system power without a keyboard.
