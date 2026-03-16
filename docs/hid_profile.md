# HID Profile

ControllerOS now uses a single Xbox One S (model 1708) BLE-compatible HID
report profile for the `hidd` daemon.

## Profile identity

Default identity values in `configs/hid/hid.toml`:

- `mode = "xbox_one_s_1708"`
- `vendor_id = 0x045e` (Microsoft)
- `product_id = 0x02fd` (Xbox One S 1708 Bluetooth profile)
- `version = 0x0408`
- `country = 0`

These values are written into the UHID create payload so BlueZ/kernel-host stacks
see Xbox-compatible identity metadata.

For BLE identity exposure, `hidd` also exports Device Information Service (`0x180A`)
with PnP ID (`0x2A50`) derived from the same config values.

PnP ID byte layout (7 bytes):

1. Vendor ID Source (`0x02` = USB-IF)
2. Vendor ID (LE, 2 bytes)
3. Product ID (LE, 2 bytes)
4. Product version (LE, 2 bytes)

## Descriptor

- Source reference: `ESP32-BLE-CompositeHID/XboxDescriptors.h` (`XboxOneS_1708_HIDDescriptor`)
- Descriptor length: `334` bytes
- Single HID interface
- Report IDs:
  - `0x01` main gamepad input report
  - `0x02` extra consumer input report
  - `0x03` output report (rumble/effects)
  - `0x04` status-style input report

BLE HOG report characteristic topology now mirrors this descriptor:

- Input report characteristic + Report Reference (`0x2908`) for `0x01`
- Input report characteristic + Report Reference (`0x2908`) for `0x02`
- Output report characteristic + Report Reference (`0x2908`) for `0x03`
- Input report characteristic + Report Reference (`0x2908`) for `0x04`

## Input report format (`0x01`)

Total length: `17` bytes (including report ID).

Byte layout:

1. Report ID (`0x01`)
2. `LX` (`u16`, centered at `0x8000`)
3. `LY` (`u16`, centered at `0x8000`)
4. `RX` (`u16`, centered at `0x8000`)
5. `RY` (`u16`, centered at `0x8000`)
6. `LT` (`u16`, clamped to `0..=1023`)
7. `RT` (`u16`, clamped to `0..=1023`)
8. Hat (`0..=8`, where `0` is neutral)
9. Buttons (`15` bits)
10. Consumer bit (`share/back-style bit)

`hidd` synthetic patterns currently drive this input report.

### BLE HOG framing boundary (03.5 requirement #1)

- UHID input path remains report-ID based (`0x01` prefix in `UHID_INPUT2` bytes).
- BLE HID Report characteristic values are payload-only; the report ID is conveyed
  by the `0x2908` Report Reference descriptor.
- `hidd` strips input report ID bytes before serving GATT `ReadValue`/notify data.
- `hidd` accepts BLE output writes as payload-only and normalizes them to
  report-ID-prefixed bytes for existing output parser/logging paths.

HID characteristics/descriptors use encrypted access flags (`encrypt-read`,
`encrypt-write`, `encrypt-notify`) for bonded-link operation.

## Output report handling (MVP)

`hidd` now drains UHID events and handles host output traffic safely:

- `UHID_OUTPUT`: parsed/logged/dropped (no haptics implemented in MVP)
- `UHID_SET_REPORT`: acknowledged and dropped
- `UHID_GET_REPORT`: replied with not-supported status

This prevents daemon stalls from unread output traffic while keeping MVP scope.

## BLE pairing agent

`hidd` registers a `NoInputNoOutput` BlueZ pairing agent via D-Bus
(`org.bluez.Agent1` at `/org/controlleros/agent`) during GATT HOG registration.
This enables automatic "Just Works" BLE pairing without requiring a separate
init script or persistent `bluetoothctl` process. The agent:

- Auto-accepts `RequestAuthorization` and `AuthorizeService` callbacks
- Rejects pin/passkey/confirmation requests (not applicable for Just Works)
- Is registered as the default agent via `AgentManager1.RequestDefaultAgent`
- Is unregistered when hidd shuts down

## Host behavior notes

- Hosts may label the controller differently by platform, but with this profile
  and identity values it should map as an Xbox-compatible BLE gamepad class.
- Exact displayed name can vary (for example, generic gamepad naming on some
  Linux/macOS stacks).
- ControllerOS now advertises BLE HOGP (`0x1812`) from `hidd` and the adapter
  local name defaults to `ControllerOS Xbox Controller` via
  `configs/bluez/main.conf`.
