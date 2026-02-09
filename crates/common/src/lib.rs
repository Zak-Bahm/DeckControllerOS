#![forbid(unsafe_code)]

pub mod hid;

#[cfg(test)]
mod tests {
    use super::hid::{InputReport, HID_REPORT_DESCRIPTOR, REPORT_ID};

    #[test]
    fn descriptor_is_not_empty() {
        assert!(!HID_REPORT_DESCRIPTOR.is_empty());
    }

    #[test]
    fn report_serializes_with_report_id_prefix() {
        let report = InputReport::default();
        let bytes = report.to_bytes();
        assert_eq!(bytes[0], REPORT_ID);
    }
}
