//! Shared HID profile constants and report packing for checkpoint 03.

use serde::Deserialize;

pub const XBOX_VENDOR_ID: u16 = 0x045e;
pub const XBOX_ONE_S_1708_PRODUCT_ID: u16 = 0x02fd;
pub const XBOX_ONE_S_1708_VERSION: u16 = 0x0408;
pub const XBOX_COUNTRY_CODE: u16 = 0x0000;

pub const XBOX_INPUT_REPORT_ID: u8 = 0x01;
pub const XBOX_EXTRA_INPUT_REPORT_ID: u8 = 0x02;
pub const XBOX_OUTPUT_REPORT_ID: u8 = 0x03;
pub const XBOX_STATUS_INPUT_REPORT_ID: u8 = 0x04;
pub const XBOX_BUTTON_A: u16 = 0x0001;
pub const XBOX_BUTTON_B: u16 = 0x0002;
pub const XBOX_BUTTON_X: u16 = 0x0008;
pub const XBOX_BUTTON_Y: u16 = 0x0010;
pub const XBOX_BUTTON_LB: u16 = 0x0040;
pub const XBOX_BUTTON_RB: u16 = 0x0080;
pub const XBOX_BUTTON_SELECT: u16 = 0x0400;
pub const XBOX_BUTTON_START: u16 = 0x0800;
pub const XBOX_BUTTON_HOME: u16 = 0x1000;
pub const XBOX_BUTTON_LS: u16 = 0x2000;
pub const XBOX_BUTTON_RS: u16 = 0x4000;

pub const XBOX_BUTTON_MASK: u16 = 0x7fff;
pub const XBOX_BUTTON_MAX_INDEX: u8 = 14;

pub const XBOX_STICK_MIN: i16 = -32768;
pub const XBOX_STICK_MAX: i16 = 32767;
pub const XBOX_TRIGGER_MIN: u16 = 0;
pub const XBOX_TRIGGER_MAX: u16 = 1023;
pub const XBOX_HAT_MIN: u8 = 0;
pub const XBOX_HAT_MAX: u8 = 8;

pub const XBOX_INPUT_REPORT_LEN: usize = 17;
pub const XBOX_INPUT_PAYLOAD_LEN: usize = XBOX_INPUT_REPORT_LEN - 1;
pub const XBOX_EXTRA_INPUT_PAYLOAD_LEN: usize = 1;
pub const XBOX_STATUS_INPUT_PAYLOAD_LEN: usize = 1;
pub const XBOX_OUTPUT_PAYLOAD_LEN: usize = 8;
pub const XBOX_OUTPUT_REPORT_LEN: usize = 1 + XBOX_OUTPUT_PAYLOAD_LEN;

pub const XBOX_ONE_S_1708_HID_REPORT_DESCRIPTOR: [u8; 334] = [
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x01, 0xa1, 0x00, 0x09, 0x30, 0x09, 0x31,
    0x15, 0x00, 0x27, 0xff, 0xff, 0x00, 0x00, 0x95, 0x02, 0x75, 0x10, 0x81, 0x02, 0xc0, 0x09, 0x01,
    0xa1, 0x00, 0x09, 0x32, 0x09, 0x35, 0x15, 0x00, 0x27, 0xff, 0xff, 0x00, 0x00, 0x95, 0x02, 0x75,
    0x10, 0x81, 0x02, 0xc0, 0x05, 0x02, 0x09, 0xc5, 0x15, 0x00, 0x26, 0xff, 0x03, 0x95, 0x01, 0x75,
    0x0a, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00, 0x75, 0x06, 0x95, 0x01, 0x81, 0x03, 0x05, 0x02, 0x09,
    0xc4, 0x15, 0x00, 0x26, 0xff, 0x03, 0x95, 0x01, 0x75, 0x0a, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00,
    0x75, 0x06, 0x95, 0x01, 0x81, 0x03, 0x05, 0x01, 0x09, 0x39, 0x15, 0x01, 0x25, 0x08, 0x35, 0x00,
    0x46, 0x3b, 0x01, 0x66, 0x14, 0x00, 0x75, 0x04, 0x95, 0x01, 0x81, 0x42, 0x75, 0x04, 0x95, 0x01,
    0x15, 0x00, 0x25, 0x00, 0x35, 0x00, 0x45, 0x00, 0x65, 0x00, 0x81, 0x03, 0x05, 0x09, 0x19, 0x01,
    0x29, 0x0f, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x0f, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00,
    0x75, 0x01, 0x95, 0x01, 0x81, 0x03, 0x05, 0x0c, 0x0a, 0x24, 0x02, 0x15, 0x00, 0x25, 0x01, 0x95,
    0x01, 0x75, 0x01, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00, 0x75, 0x07, 0x95, 0x01, 0x81, 0x03, 0x05,
    0x0c, 0x09, 0x01, 0x85, 0x02, 0xa1, 0x01, 0x05, 0x0c, 0x0a, 0x23, 0x02, 0x15, 0x00, 0x25, 0x01,
    0x95, 0x01, 0x75, 0x01, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00, 0x75, 0x07, 0x95, 0x01, 0x81, 0x03,
    0xc0, 0x05, 0x0f, 0x09, 0x21, 0x85, 0x03, 0xa1, 0x02, 0x09, 0x97, 0x15, 0x00, 0x25, 0x01, 0x75,
    0x04, 0x95, 0x01, 0x91, 0x02, 0x15, 0x00, 0x25, 0x00, 0x75, 0x04, 0x95, 0x01, 0x91, 0x03, 0x09,
    0x70, 0x15, 0x00, 0x25, 0x64, 0x75, 0x08, 0x95, 0x04, 0x91, 0x02, 0x09, 0x50, 0x66, 0x01, 0x10,
    0x55, 0x0e, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x91, 0x02, 0x09, 0xa7, 0x15,
    0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x91, 0x02, 0x65, 0x00, 0x55, 0x00, 0x09, 0x7c,
    0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x91, 0x02, 0xc0, 0x05, 0x06, 0x09, 0x20,
    0x85, 0x04, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02, 0xc0,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HidProfileMode {
    #[serde(rename = "xbox_one_s_1708")]
    XboxOneS1708,
}

impl HidProfileMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::XboxOneS1708 => "xbox_one_s_1708",
        }
    }

    pub const fn report_descriptor(self) -> &'static [u8] {
        match self {
            Self::XboxOneS1708 => &XBOX_ONE_S_1708_HID_REPORT_DESCRIPTOR,
        }
    }

    pub const fn input_report_len(self) -> usize {
        match self {
            Self::XboxOneS1708 => XBOX_INPUT_REPORT_LEN,
        }
    }

    pub const fn input_report_id(self) -> u8 {
        match self {
            Self::XboxOneS1708 => XBOX_INPUT_REPORT_ID,
        }
    }
}

impl Default for HidProfileMode {
    fn default() -> Self {
        Self::XboxOneS1708
    }
}

/// Default profile descriptor exported for compatibility with checkpoint-03 callers.
pub const HID_REPORT_DESCRIPTOR: [u8; 334] = XBOX_ONE_S_1708_HID_REPORT_DESCRIPTOR;

/// Input report length, including the report ID byte.
pub const INPUT_REPORT_LEN: usize = XBOX_INPUT_REPORT_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputReport {
    pub buttons: u16,
    pub hat: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
    pub lt: u16,
    pub rt: u16,
    pub share: u8,
}

impl Default for InputReport {
    fn default() -> Self {
        Self {
            buttons: 0,
            hat: XBOX_HAT_MIN,
            lx: 0,
            ly: 0,
            rx: 0,
            ry: 0,
            lt: XBOX_TRIGGER_MIN,
            rt: XBOX_TRIGGER_MIN,
            share: 0,
        }
    }
}

impl InputReport {
    pub fn to_bytes(self) -> [u8; INPUT_REPORT_LEN] {
        let lx = stick_to_wire(self.lx).to_le_bytes();
        let ly = stick_to_wire(self.ly).to_le_bytes();
        let rx = stick_to_wire(self.rx).to_le_bytes();
        let ry = stick_to_wire(self.ry).to_le_bytes();
        let lt = trigger_to_wire(self.lt).to_le_bytes();
        let rt = trigger_to_wire(self.rt).to_le_bytes();
        let buttons = self.buttons & XBOX_BUTTON_MASK;

        [
            XBOX_INPUT_REPORT_ID,
            lx[0],
            lx[1],
            ly[0],
            ly[1],
            rx[0],
            rx[1],
            ry[0],
            ry[1],
            lt[0],
            lt[1],
            rt[0],
            rt[1],
            self.hat.min(XBOX_HAT_MAX) & 0x0f,
            (buttons & 0x00ff) as u8,
            ((buttons >> 8) & 0x007f) as u8,
            self.share & 0x01,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutputReport {
    pub dc_enable_actuators: u8,
    pub left_trigger_magnitude: u8,
    pub right_trigger_magnitude: u8,
    pub weak_motor_magnitude: u8,
    pub strong_motor_magnitude: u8,
    pub duration: u8,
    pub start_delay: u8,
    pub loop_count: u8,
}

impl OutputReport {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() != XBOX_OUTPUT_REPORT_LEN || data[0] != XBOX_OUTPUT_REPORT_ID {
            return None;
        }

        Some(Self {
            dc_enable_actuators: data[1],
            left_trigger_magnitude: data[2],
            right_trigger_magnitude: data[3],
            weak_motor_magnitude: data[4],
            strong_motor_magnitude: data[5],
            duration: data[6],
            start_delay: data[7],
            loop_count: data[8],
        })
    }
}

const XBOX_AXIS_CENTER_OFFSET: i32 = 0x8000;

fn stick_to_wire(value: i16) -> u16 {
    (i32::from(value) + XBOX_AXIS_CENTER_OFFSET).clamp(0, 0xffff) as u16
}

fn trigger_to_wire(value: u16) -> u16 {
    value.clamp(XBOX_TRIGGER_MIN, XBOX_TRIGGER_MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        InputReport, OutputReport, HID_REPORT_DESCRIPTOR, INPUT_REPORT_LEN, XBOX_BUTTON_MASK,
        XBOX_INPUT_REPORT_ID, XBOX_OUTPUT_REPORT_ID, XBOX_TRIGGER_MAX,
    };

    #[test]
    fn descriptor_has_expected_length_and_report_ids() {
        assert_eq!(HID_REPORT_DESCRIPTOR.len(), 334);
        assert!(HID_REPORT_DESCRIPTOR
            .windows(2)
            .any(|w| w == [0x85, XBOX_INPUT_REPORT_ID]));
        assert!(HID_REPORT_DESCRIPTOR
            .windows(2)
            .any(|w| w == [0x85, XBOX_OUTPUT_REPORT_ID]));
    }

    #[test]
    fn input_report_serialization_matches_expected_length() {
        let bytes = InputReport::default().to_bytes();
        assert_eq!(bytes.len(), INPUT_REPORT_LEN);
        assert_eq!(bytes[0], XBOX_INPUT_REPORT_ID);
    }

    #[test]
    fn input_report_masks_clamps_and_centers_fields() {
        let report = InputReport {
            buttons: 0xffff,
            hat: 0xff,
            lx: 0,
            ly: 0,
            rx: 0,
            ry: 0,
            lt: u16::MAX,
            rt: u16::MAX,
            share: 0xff,
        };

        let bytes = report.to_bytes();

        // Sticks centered around 0x8000.
        assert_eq!(u16::from_le_bytes([bytes[1], bytes[2]]), 0x8000);
        assert_eq!(u16::from_le_bytes([bytes[3], bytes[4]]), 0x8000);
        assert_eq!(u16::from_le_bytes([bytes[5], bytes[6]]), 0x8000);
        assert_eq!(u16::from_le_bytes([bytes[7], bytes[8]]), 0x8000);

        let lt = u16::from_le_bytes([bytes[9], bytes[10]]);
        let rt = u16::from_le_bytes([bytes[11], bytes[12]]);
        assert_eq!(lt, XBOX_TRIGGER_MAX);
        assert_eq!(rt, XBOX_TRIGGER_MAX);

        let buttons = u16::from(bytes[14]) | (u16::from(bytes[15] & 0x7f) << 8);
        assert_eq!(buttons, XBOX_BUTTON_MASK);

        assert_eq!(bytes[13] & 0x0f, 0x08);
        assert_eq!(bytes[16], 0x01);
    }

    #[test]
    fn parses_output_report_payload() {
        let raw = [XBOX_OUTPUT_REPORT_ID, 1, 2, 3, 4, 5, 6, 7, 8];
        let parsed = OutputReport::parse(&raw).expect("output report should parse");

        assert_eq!(parsed.dc_enable_actuators, 1);
        assert_eq!(parsed.strong_motor_magnitude, 5);
        assert_eq!(parsed.loop_count, 8);
    }
}
