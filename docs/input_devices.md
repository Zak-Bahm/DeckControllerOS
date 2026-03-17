# Input Device Discovery and Configuration

## Overview

DeckControllerOS reads controller input from the Steam Deck's embedded controller using the kernel's hidraw interface. The `hid-steam` kernel driver creates the necessary device nodes.

## hid-steam Driver Topology

The `hid-steam` driver (`CONFIG_HID_STEAM=y`) creates four HID sub-devices when it detects the Deck's controller (VID `0x28DE`, PID `0x1205`):

| hidraw device | Type    | has `input/` subdir | Purpose                    | Used by DeckControllerOS |
|---------------|---------|---------------------|----------------------------|--------------------------|
| hidraw0       | Keyboard| Yes                 | Lizard mode keyboard events| No                       |
| hidraw1       | Mouse   | Yes                 | Lizard mode mouse events   | No                       |
| hidraw2       | Client  | **No**              | Raw controller reports     | **Yes**                  |

The client device (`client_hdev`) is the one without an `input/` subdirectory in its sysfs path. Device numbers may vary — discovery is by VID/PID and sysfs structure, never by hardcoded path.

## Discovery Process

Discovery is implemented in `crates/input/src/hidraw.rs::discover_deck_hidraw()`:

1. Scan `/sys/class/hidraw/` for all hidraw devices
2. For each device, read the `uevent` file from the parent HID device to extract VID and PID
3. Match VID `0x28DE` and PID `0x1205` (Valve Steam Deck controller)
4. Among matching devices, select the one whose sysfs path does **not** contain an `input/` subdirectory — this is the client device
5. Return the `/dev/hidrawN` path for the selected device

## Lizard Mode

"Lizard mode" is the Deck controller's default behavior where it emulates a keyboard and mouse (for desktop use without Steam). It must be disabled for raw controller input to work correctly.

### Disabling Lizard Mode

Two HID feature reports are sent via `HIDIOCSFEATURE` ioctl on the client hidraw device:

**Report 1 — Clear Digital Mappings (0x81):**
```
[0x00, 0x81, ...zero-padded to 65 bytes]
```
Stops keyboard/mouse emulation from the controller.

**Report 2 — Set Settings Values (0x87):**
```
[0x00, 0x87, 0x03, 0x28,
 SETTING_LEFT_TRACKPAD_MODE (7),    0x00, TRACKPAD_NONE (7),    0x00,
 SETTING_RIGHT_TRACKPAD_MODE (8),   0x00, TRACKPAD_NONE (7),    0x00,
 SETTING_LEFT_TRACKPAD_CLICK_PRESSURE (52),  0x00, 0xFF, 0x7F,
 SETTING_RIGHT_TRACKPAD_CLICK_PRESSURE (53), 0x00, 0xFF, 0x7F,
 SETTING_STEAM_WATCHDOG_ENABLE (71), 0x00, 0x00, 0x00,
 ...zero-padded to 65 bytes]
```
This matches the kernel's `steam_set_lizard_mode(false)` implementation:
- Disables both trackpad modes
- Sets max click pressure (effectively disabling trackpad clicks)
- Disables the Steam watchdog (which would re-enable lizard mode)

### Why hidraw Instead of evdev

The `hid-steam` driver has two guards preventing evdev input on the Deck:

1. **`steam_input_open()`** skips `steam_set_lizard_mode(false)` for `STEAM_QUIRK_DECK` devices
2. **`steam_do_deck_input_event()`** checks a `gamepad_mode` flag (defaults to false) and returns early without emitting events

The hidraw approach bypasses both guards by reading raw HID data directly from the client device.

## Kernel Requirements

The following kernel configs must be enabled in `configs/kernel/steamdeck_defconfig`:

```
CONFIG_HID=y
CONFIG_HID_GENERIC=y
CONFIG_HID_STEAM=y
CONFIG_HIDRAW=y
CONFIG_INPUT_JOYDEV=y
```

`CONFIG_HIDRAW=y` is required for `/sys/class/hidraw/` and `/dev/hidraw*` device nodes to exist.

## Troubleshooting

### No `/sys/class/hidraw/` directory
- `CONFIG_HIDRAW=y` is missing from the kernel config. Rebuild the kernel.

### No Valve devices in hidraw
- Check `CONFIG_HID_STEAM=y` is enabled.
- Verify the controller is detected: `dmesg | grep -i steam`
- Check for VID/PID: look for `28DE:1205` in `/sys/class/hidraw/*/device/uevent`

### hidraw device found but no reports received
- Ensure you're opening the **client** device (no `input/` subdirectory), not the keyboard or mouse device.
- Check that another process hasn't already opened the client device (only one client is supported by hid-steam).

### Lizard mode not disabled (keyboard/mouse events still happening)
- Verify the feature reports are being sent successfully (check hidd logs for errors).
- Ensure the hidraw device is opened for both read and write (write is needed for `HIDIOCSFEATURE`).

### `controllerosctl input list` shows devices but `input monitor` fails
- `input list` uses evdev for diagnostics. `input monitor` uses hidraw.
- These are independent subsystems. hidraw discovery failure doesn't affect evdev listing.

### Device numbers changed after reboot
- This is normal. Discovery is by VID/PID and sysfs structure, not by device number. No action needed.
