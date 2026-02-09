#![forbid(unsafe_code)]

pub mod config;
pub mod hid;

#[cfg(test)]
mod tests {
    use super::hid::{InputReport, HID_REPORT_DESCRIPTOR, REPORT_ID};

    #[test]
    fn descriptor_has_expected_header_usage_page() {
        assert_eq!(HID_REPORT_DESCRIPTOR[0], 0x05);
    }

    #[test]
    fn report_serializes_with_report_id_prefix() {
        let report = InputReport::default();
        let bytes = report.to_bytes();
        assert_eq!(bytes[0], REPORT_ID);
    }
}
