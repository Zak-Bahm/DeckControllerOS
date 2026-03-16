use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::hid::{
    HidProfileMode, XBOX_BUTTON_MAX_INDEX, XBOX_COUNTRY_CODE, XBOX_ONE_S_1708_PRODUCT_ID,
    XBOX_ONE_S_1708_VERSION, XBOX_TRIGGER_MAX, XBOX_VENDOR_ID,
};

pub const DEFAULT_HID_CONFIG_PATH: &str = "/etc/controlleros/hid.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HidConfig {
    pub device: DeviceConfig,
    #[serde(default)]
    pub profile: ProfileConfig,
    pub report: ReportConfig,
    pub pattern: PatternConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProfileConfig {
    #[serde(default)]
    pub mode: HidProfileMode,
    #[serde(default = "default_profile_vendor_id")]
    pub vendor_id: u16,
    #[serde(default = "default_profile_product_id")]
    pub product_id: u16,
    #[serde(default = "default_profile_version")]
    pub version: u16,
    #[serde(default = "default_profile_country")]
    pub country: u16,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            mode: HidProfileMode::default(),
            vendor_id: default_profile_vendor_id(),
            product_id: default_profile_product_id(),
            version: default_profile_version(),
            country: default_profile_country(),
        }
    }
}

fn default_profile_vendor_id() -> u16 {
    XBOX_VENDOR_ID
}

fn default_profile_product_id() -> u16 {
    XBOX_ONE_S_1708_PRODUCT_ID
}

fn default_profile_version() -> u16 {
    XBOX_ONE_S_1708_VERSION
}

fn default_profile_country() -> u16 {
    XBOX_COUNTRY_CODE
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
        if self.profile.vendor_id == 0 {
            return Err(HidConfigError::Validation(
                "profile.vendor_id must be non-zero".to_string(),
            ));
        }
        if self.profile.product_id == 0 {
            return Err(HidConfigError::Validation(
                "profile.product_id must be non-zero".to_string(),
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
                if button_index > XBOX_BUTTON_MAX_INDEX {
                    return Err(HidConfigError::Validation(format!(
                        "pattern.button_index must be in 0..={XBOX_BUTTON_MAX_INDEX}"
                    )));
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
                        if min < 0 || max > i16::try_from(XBOX_TRIGGER_MAX).expect("constant fits")
                        {
                            return Err(HidConfigError::Validation(format!(
                                "trigger sweep range must be within 0..={XBOX_TRIGGER_MAX}"
                            )));
                        }
                    }
                    AxisName::Lx | AxisName::Ly | AxisName::Rx | AxisName::Ry => {}
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AxisName, HidConfig, HidProfileMode, PatternConfig};

    #[test]
    fn parses_valid_button_toggle_config() {
        let cfg = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Xbox Controller"

            [profile]
            mode = "xbox_one_s_1708"
            vendor_id = 0x045e
            product_id = 0x02fd
            version = 0x0408
            country = 0

            [report]
            rate_hz = 125

            [pattern]
            kind = "button_toggle"
            button_index = 0
            period_reports = 30
            "#,
        )
        .expect("config should parse");

        assert_eq!(cfg.profile.mode, HidProfileMode::XboxOneS1708);
        assert_eq!(cfg.profile.vendor_id, 0x045e);
        assert_eq!(cfg.profile.product_id, 0x02fd);
        assert_eq!(cfg.profile.version, 0x0408);
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
    fn uses_default_profile_identity_when_omitted() {
        let cfg = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Xbox Controller"

            [report]
            rate_hz = 125

            [pattern]
            kind = "button_toggle"
            button_index = 0
            period_reports = 30
            "#,
        )
        .expect("config should parse");

        assert_eq!(cfg.profile.mode, HidProfileMode::XboxOneS1708);
        assert_eq!(cfg.profile.vendor_id, 0x045e);
        assert_eq!(cfg.profile.product_id, 0x02fd);
        assert_eq!(cfg.profile.version, 0x0408);
        assert_eq!(cfg.profile.country, 0);
    }

    #[test]
    fn rejects_invalid_rate() {
        let err = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Xbox Controller"

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
            name = "ControllerOS Xbox Controller"

            [report]
            rate_hz = 60

            [pattern]
            kind = "axis_sweep"
            axis = "rt"
            step = 5
            min = 0
            max = 2000
            "#,
        )
        .expect_err("out-of-range sweep should fail");

        assert!(
            err.to_string().contains("trigger sweep range"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_button_index_above_xbox_range() {
        let err = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Xbox Controller"

            [report]
            rate_hz = 120

            [pattern]
            kind = "button_toggle"
            button_index = 15
            period_reports = 10
            "#,
        )
        .expect_err("button index above 14 should fail");

        assert!(
            err.to_string().contains("pattern.button_index"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parses_valid_axis_sweep_config() {
        let cfg = HidConfig::from_toml_str(
            r#"
            [device]
            name = "ControllerOS Xbox Controller"

            [report]
            rate_hz = 250

            [pattern]
            kind = "axis_sweep"
            axis = "rt"
            step = 8
            min = 0
            max = 1023
            "#,
        )
        .expect("axis sweep config should parse");

        assert_eq!(cfg.report.rate_hz, 250);
        assert_eq!(
            cfg.pattern,
            PatternConfig::AxisSweep {
                axis: AxisName::Rt,
                step: 8,
                min: 0,
                max: 1023
            }
        );
    }
}
