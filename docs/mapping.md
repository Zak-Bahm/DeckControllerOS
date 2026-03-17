# Input Mapping: Steam Deck → Xbox-Style HID

## Overview

DeckControllerOS reads raw HID reports from the Steam Deck's embedded controller via hidraw and maps them into Xbox One S 1708-compatible HID reports transmitted over BLE GATT HOG.

## Raw Report Format

The Deck controller sends 64-byte reports with type `0x09` (`ID_CONTROLLER_DECK_STATE`). The report header is at data[0..3]: `[0x01, 0x00, 0x09, ...]`.

### Button Bit Positions

| Byte     | Bit | Deck Button        | Xbox HID Field |
|----------|-----|--------------------|----------------|
| data[8]  | 7   | A                  | A              |
| data[8]  | 6   | X                  | X              |
| data[8]  | 5   | B                  | B              |
| data[8]  | 4   | Y                  | Y              |
| data[8]  | 3   | Left bumper        | LB             |
| data[8]  | 2   | Right bumper       | RB             |
| data[9]  | 0   | D-pad up           | Hat N          |
| data[9]  | 1   | D-pad right        | Hat E          |
| data[9]  | 2   | D-pad left         | Hat W          |
| data[9]  | 3   | D-pad down         | Hat S          |
| data[9]  | 4   | ≡ (three lines)    | Back/Select    |
| data[9]  | 5   | Steam button       | Home/Guide     |
| data[9]  | 6   | ☰ (hamburger)      | Start/Menu     |
| data[10] | 6   | Left stick click   | LS             |
| data[11] | 2   | Right stick click  | RS             |

### Axis Byte Offsets

All axis values are little-endian signed 16-bit integers.

| Offset       | Deck Axis       | Raw Range         | Xbox HID Field | Output Range |
|--------------|-----------------|-------------------|----------------|--------------|
| data[48..50] | Left stick X    | -32768 .. 32767   | LX             | -32768 .. 32767 |
| data[50..52] | Left stick Y    | -32768 .. 32767   | LY             | -32768 .. 32767 |
| data[52..54] | Right stick X   | -32768 .. 32767   | RX             | -32768 .. 32767 |
| data[54..56] | Right stick Y   | -32768 .. 32767   | RY             | -32768 .. 32767 |
| data[44..46] | Left trigger    | 0 .. 32767        | LT             | 0 .. 1023       |
| data[46..48] | Right trigger   | 0 .. 32767        | RT             | 0 .. 1023       |

### Y-Axis Negation

The Deck's raw stick Y values use Y-up-positive convention. The Xbox HID descriptor uses Y-down-positive (standard gamepad convention). The reader negates Y values after reading.

## Normalization

### Stick Axes (LX, LY, RX, RY)

1. Apply radial deadzone (default: 4000 out of 32767)
2. If magnitude < deadzone → output 0
3. Otherwise, rescale linearly: `(magnitude - deadzone) / (32767 - deadzone) * 32767`
4. Preserve direction

### Triggers (LT, RT)

1. No deadzone applied
2. Scale from raw range (0..32767) to output range (0..1023)

### D-Pad

D-pad is encoded as a hat switch value (0-8, where 0 = neutral):
- Individual direction bits are read and combined
- Diagonal combinations (NE, SE, SW, NW) are supported

## Ignored Controls

The following Deck inputs are intentionally not mapped:

| Control           | Reason                                    |
|-------------------|-------------------------------------------|
| Left trackpad     | No Xbox equivalent; lizard mode disabled  |
| Right trackpad    | No Xbox equivalent; lizard mode disabled  |
| Left rear button  | No Xbox equivalent                        |
| Right rear button | No Xbox equivalent                        |
| Gyro/IMU          | No Xbox equivalent                        |
| Touchscreen       | Not a controller input                    |
| Haptics           | Output device, not input                  |

These controls produce no changes in the Xbox HID report.

## Configuration

Mapping parameters are configured in `configs/mapping/xbox.toml` (installed to `/etc/controlleros/mapping/xbox.toml`).

Key parameters:
- `deadzone`: Per-axis deadzone threshold (default: 4000 for sticks, 0 for triggers)
- `source_range_min` / `source_range_max`: Raw input range
- `output_range_min` / `output_range_max`: Mapped output range
