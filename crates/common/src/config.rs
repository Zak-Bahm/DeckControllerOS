use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

pub const DEFAULT_HID_CONFIG_PATH: &str = "/etc/controlleros/hid.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HidConfig {
    pub device: DeviceConfig,
    pub report: ReportConfig,
    pub pattern: PatternConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceConfig {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReportConfig {
    pub rate_hz: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PatternConfig {
    ButtonToggle {
        button_index: u8,
        period_reports: u16,
    },
    AxisSweep {
        axis: AxisName,
        step: i16,
        min: i16,
        max: i16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisName {
    Lx,
    Ly,
    Rx,
    Ry,
    Lt,
    Rt,
}

#[derive(Debug, Error)]
pub enum HidConfigError {
    #[error("failed to read config at {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML in {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid HID config: {0}")]
    Validation(String),
}

impl HidConfig {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, HidConfigError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();
        let raw = fs::read_to_string(path).map_err(|source| HidConfigError::Read {
            path: path_str.clone(),
            source,
        })?;
        let parsed: Self = toml::from_str(&raw).map_err(|source| HidConfigError::Parse {
            path: path_str,
            source,
        })?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn from_toml_str(raw: &str) -> Result<Self, HidConfigError> {
        let parsed: Self = toml::from_str(raw).map_err(|source| HidConfigError::Parse {
            path: "<inline>".to_string(),
            source,
        })?;
        parsed.validate()?;
        Ok(parsed)
    }

    fn validate(&self) -> Result<(), HidConfigError> {
        if self.device.name.trim().is_empty() {
            return Err(HidConfigError::Validation(
                "device.name must not be empty".to_string(),
            ));
        }
        if self.report.rate_hz == 0 || self.report.rate_hz > 1000 {
            return Err(HidConfigError::Validation(
                "report.rate_hz must be in 1..=1000".to_string(),
            ));
        }
        match self.pattern {
            PatternConfig::ButtonToggle {
                button_index,
                period_reports,
            } => {
                if button_index > 9 {
                    return Err(HidConfigError::Validation(
                        "pattern.button_index must be in 0..=9".to_string(),
                    ));
                }
                if period_reports == 0 {
                    return Err(HidConfigError::Validation(
                        "pattern.period_reports must be greater than zero".to_string(),
                    ));
                }
            }
            PatternConfig::AxisSweep {
                axis,
                step,
                min,
                max,
            } => {
                if step <= 0 {
                    return Err(HidConfigError::Validation(
                        "pattern.step must be greater than zero".to_string(),
                    ));
                }
                if min >= max {
                    return Err(HidConfigError::Validation(
                        "pattern.min must be less than pattern.max".to_string(),
                    ));
                }
                match axis {
                    AxisName::Lt | AxisName::Rt => {
                        if min < 0 || max > 255 {
                            return Err(HidConfigError::Validation(
                                "trigger sweep range must be within 0..=255".to_string(),
                            ));
                        }
                    }
                    AxisName::Lx | AxisName::Ly | AxisName::Rx | AxisName::Ry => {
                        if min < -127 || max > 127 {
                            return Err(HidConfigError::Validation(
                                "stick sweep range must be within -127..=127".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AxisName, HidConfig, PatternConfig};

    #[test]
    fn parses_valid_button_toggle_config() {
        let cfg = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Gamepad"
            vendor_id = 0x28de
            product_id = 0x1205

            [report]
            rate_hz = 125

            [pattern]
            kind = "button_toggle"
            button_index = 0
            period_reports = 30
            "#,
        )
        .expect("config should parse");

        assert_eq!(cfg.device.vendor_id, 0x28de);
        assert_eq!(cfg.device.product_id, 0x1205);
        assert_eq!(cfg.report.rate_hz, 125);
        assert_eq!(
            cfg.pattern,
            PatternConfig::ButtonToggle {
                button_index: 0,
                period_reports: 30
            }
        );
    }

    #[test]
    fn rejects_invalid_rate() {
        let err = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Gamepad"
            vendor_id = 1
            product_id = 2

            [report]
            rate_hz = 0

            [pattern]
            kind = "button_toggle"
            button_index = 0
            period_reports = 1
            "#,
        )
        .expect_err("rate_hz=0 should fail");

        assert!(
            err.to_string().contains("report.rate_hz"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_axis_sweep_out_of_range() {
        let err = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Gamepad"
            vendor_id = 1
            product_id = 2

            [report]
            rate_hz = 60

            [pattern]
            kind = "axis_sweep"
            axis = "lx"
            step = 5
            min = -200
            max = 100
            "#,
        )
        .expect_err("out-of-range sweep should fail");

        assert!(
            err.to_string().contains("stick sweep range"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parses_valid_axis_sweep_config() {
        let cfg = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Gamepad"
            vendor_id = 1
            product_id = 2

            [report]
            rate_hz = 250

            [pattern]
            kind = "axis_sweep"
            axis = "rt"
            step = 4
            min = 0
            max = 255
            "#,
        )
        .expect("axis sweep config should parse");

        assert_eq!(cfg.report.rate_hz, 250);
        assert_eq!(
            cfg.pattern,
            PatternConfig::AxisSweep {
                axis: AxisName::Rt,
                step: 4,
                min: 0,
                max: 255
            }
        );
    }
}
