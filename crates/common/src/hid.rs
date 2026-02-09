//! Shared HID profile constants and report packing for checkpoint 03.

pub const REPORT_ID: u8 = 0x01;

/// Bits 0..=9 are used by the checkpoint-03 gamepad profile.
pub const BUTTON_MASK: u16 = 0x03ff;

/// Input report length, including the report ID byte.
pub const INPUT_REPORT_LEN: usize = 10;

/// Simple gamepad descriptor for one input report:
/// - 10 buttons
/// - hat switch
/// - LX/LY/RX/RY as signed 8-bit
/// - LT/RT as unsigned 8-bit
pub const HID_REPORT_DESCRIPTOR: [u8; 92] = [
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x05, // Usage (Game Pad)
    0xa1, 0x01, // Collection (Application)
    0x85, REPORT_ID, // Report ID
    0x05, 0x09, // Usage Page (Button)
    0x19, 0x01, // Usage Minimum (Button 1)
    0x29, 0x0a, // Usage Maximum (Button 10)
    0x15, 0x00, // Logical Minimum (0)
    0x25, 0x01, // Logical Maximum (1)
    0x95, 0x0a, // Report Count (10)
    0x75, 0x01, // Report Size (1)
    0x81, 0x02, // Input (Data,Var,Abs)
    0x95, 0x01, // Report Count (1)
    0x75, 0x06, // Report Size (6)
    0x81, 0x03, // Input (Const,Var,Abs) padding
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x39, // Usage (Hat switch)
    0x15, 0x00, // Logical Minimum (0)
    0x25, 0x07, // Logical Maximum (7)
    0x35, 0x00, // Physical Minimum (0)
    0x46, 0x3b, 0x01, // Physical Maximum (315)
    0x65, 0x14, // Unit (English Rotation, Degrees)
    0x75, 0x04, // Report Size (4)
    0x95, 0x01, // Report Count (1)
    0x81, 0x02, // Input (Data,Var,Abs)
    0x75, 0x04, // Report Size (4)
    0x95, 0x01, // Report Count (1)
    0x81, 0x03, // Input (Const,Var,Abs) padding
    0x09, 0x30, // Usage (X)
    0x09, 0x31, // Usage (Y)
    0x09, 0x33, // Usage (Rx)
    0x09, 0x34, // Usage (Ry)
    0x15, 0x81, // Logical Minimum (-127)
    0x25, 0x7f, // Logical Maximum (127)
    0x75, 0x08, // Report Size (8)
    0x95, 0x04, // Report Count (4)
    0x81, 0x02, // Input (Data,Var,Abs)
    0x05, 0x02, // Usage Page (Simulation Controls)
    0x09, 0xc5, // Usage (Brake)
    0x09, 0xc4, // Usage (Accelerator)
    0x15, 0x00, // Logical Minimum (0)
    0x25, 0xff, // Logical Maximum (255)
    0x75, 0x08, // Report Size (8)
    0x95, 0x02, // Report Count (2)
    0x81, 0x02, // Input (Data,Var,Abs)
    0xc0, // End Collection
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputReport {
    pub buttons: u16,
    pub hat: u8,
    pub lx: i8,
    pub ly: i8,
    pub rx: i8,
    pub ry: i8,
    pub lt: u8,
    pub rt: u8,
}

impl InputReport {
    pub fn to_bytes(self) -> [u8; INPUT_REPORT_LEN] {
        let buttons = self.buttons & BUTTON_MASK;
        [
            REPORT_ID,
            (buttons & 0xff) as u8,
            (buttons >> 8) as u8 & 0x03,
            self.hat & 0x0f,
            self.lx as u8,
            self.ly as u8,
            self.rx as u8,
            self.ry as u8,
            self.lt,
            self.rt,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{InputReport, BUTTON_MASK, HID_REPORT_DESCRIPTOR, INPUT_REPORT_LEN, REPORT_ID};

    #[test]
    fn descriptor_has_expected_length_and_report_id() {
        assert_eq!(HID_REPORT_DESCRIPTOR.len(), 92);
        assert!(HID_REPORT_DESCRIPTOR
            .windows(2)
            .any(|w| w == [0x85, REPORT_ID]));
    }

    #[test]
    fn input_report_serialization_matches_expected_length() {
        let bytes = InputReport::default().to_bytes();
        assert_eq!(bytes.len(), INPUT_REPORT_LEN);
    }

    #[test]
    fn input_report_masks_buttons_and_hat_bits() {
        let report = InputReport {
            buttons: 0xffff,
            hat: 0xff,
            ..InputReport::default()
        };
        let bytes = report.to_bytes();

        let packed_buttons = u16::from(bytes[1]) | (u16::from(bytes[2] & 0x03) << 8);
        let packed_hat = bytes[3] & 0x0f;

        assert_eq!(packed_buttons, BUTTON_MASK);
        assert_eq!(packed_hat, 0x0f);
    }
}
