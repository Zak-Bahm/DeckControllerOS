#![forbid(unsafe_code)]

use serde::Deserialize;

/// Top-level mapping configuration loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct MappingConfig {
    /// Device selection criteria.
    pub device: DeviceFilter,
    /// Axis mappings from evdev to HID report fields.
    #[serde(default)]
    pub axes: Vec<AxisMapping>,
    /// Button mappings from evdev to HID report bits.
    #[serde(default)]
    pub buttons: Vec<ButtonMapping>,
}

/// Criteria for selecting which evdev device to use.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceFilter {
    /// Device name to match (exact).
    pub name: Option<String>,
    /// Vendor ID to match.
    pub vendor_id: Option<u16>,
    /// Product ID to match.
    pub product_id: Option<u16>,
}

/// Maps an evdev absolute axis to an HID report axis field.
#[derive(Debug, Clone, Deserialize)]
pub struct AxisMapping {
    /// Evdev axis code (e.g., `ABS_X` = 0x00).
    pub evdev_code: u16,
    /// Target HID axis name (e.g., "lx", "ly", "rx", "ry", "lt", "rt").
    pub hid_axis: String,
    /// Minimum value from evdev.
    pub evdev_min: i32,
    /// Maximum value from evdev.
    pub evdev_max: i32,
    /// Whether to invert the axis value.
    #[serde(default)]
    pub invert: bool,
    /// Inner deadzone radius (values within this range from center are zeroed).
    #[serde(default)]
    pub deadzone: i32,
}

/// Maps an evdev button to an HID report button bit.
#[derive(Debug, Clone, Deserialize)]
pub struct ButtonMapping {
    /// Evdev key code (e.g., `BTN_A` = 0x130).
    pub evdev_code: u16,
    /// Target HID button name (e.g., "a", "b", "x", "y", "lb", "rb").
    pub hid_button: String,
}

impl MappingConfig {
    /// Load and validate a mapping config from a TOML string.
    pub fn from_toml(s: &str) -> Result<Self, String> {
        let config: MappingConfig =
            toml::from_str(s).map_err(|e| format!("invalid mapping config: {e}"))?;
        config.validate()?;
        Ok(config)
    }

    /// Load and validate a mapping config from a file path.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        Self::from_toml(&contents)
    }

    fn validate(&self) -> Result<(), String> {
        let valid_axes = ["lx", "ly", "rx", "ry", "lt", "rt"];
        for axis in &self.axes {
            if !valid_axes.contains(&axis.hid_axis.as_str()) {
                return Err(format!("unknown hid_axis: {:?}", axis.hid_axis));
            }
            if axis.evdev_min >= axis.evdev_max {
                return Err(format!(
                    "axis {} evdev_min ({}) must be < evdev_max ({})",
                    axis.hid_axis, axis.evdev_min, axis.evdev_max
                ));
            }
        }

        let valid_buttons = [
            "a",
            "b",
            "x",
            "y",
            "lb",
            "rb",
            "back",
            "start",
            "ls",
            "rs",
            "dpad_up",
            "dpad_down",
            "dpad_left",
            "dpad_right",
        ];
        for button in &self.buttons {
            if !valid_buttons.contains(&button.hid_button.as_str()) {
                return Err(format!("unknown hid_button: {:?}", button.hid_button));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_config() {
        let toml = r#"
[device]
name = "Steam Deck"
vendor_id = 0x28DE
product_id = 0x1205

[[axes]]
evdev_code = 0x00
hid_axis = "lx"
evdev_min = -32767
evdev_max = 32767
deadzone = 4000

[[buttons]]
evdev_code = 0x130
hid_button = "a"
"#;
        let config = MappingConfig::from_toml(toml).unwrap();
        assert_eq!(config.axes.len(), 1);
        assert_eq!(config.buttons.len(), 1);
    }

    #[test]
    fn reject_unknown_axis() {
        let toml = r#"
[device]

[[axes]]
evdev_code = 0x00
hid_axis = "invalid_axis"
evdev_min = -32767
evdev_max = 32767
"#;
        let err = MappingConfig::from_toml(toml).unwrap_err();
        assert!(err.contains("unknown hid_axis"));
    }

    #[test]
    fn reject_unknown_button() {
        let toml = r#"
[device]

[[buttons]]
evdev_code = 0x130
hid_button = "turbo"
"#;
        let err = MappingConfig::from_toml(toml).unwrap_err();
        assert!(err.contains("unknown hid_button"));
    }

    #[test]
    fn reject_invalid_axis_range() {
        let toml = r#"
[device]

[[axes]]
evdev_code = 0x00
hid_axis = "lx"
evdev_min = 100
evdev_max = 100
"#;
        let err = MappingConfig::from_toml(toml).unwrap_err();
        assert!(err.contains("evdev_min"));
    }

    #[test]
    fn load_xbox_toml_from_repo() {
        let config = MappingConfig::from_file("../../configs/mapping/xbox.toml").unwrap();
        assert_eq!(config.device.name.as_deref(), Some("Steam Deck"));
        assert_eq!(config.device.vendor_id, Some(0x28DE));
        assert_eq!(config.axes.len(), 6); // lx, ly, rx, ry, lt, rt
        assert_eq!(config.buttons.len(), 14); // a,b,x,y,lb,rb,back,start,ls,rs,dpad*4
    }
}
