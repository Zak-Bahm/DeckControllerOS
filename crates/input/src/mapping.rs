#![forbid(unsafe_code)]

use serde::Deserialize;

/// Top-level mapping configuration loaded from TOML.
///
/// Button and d-pad bit positions are fixed by the Deck's hidraw report
/// layout (see `reader::parse_deck_report` and `docs/mapping.md`); only
/// axis normalization is configurable.
#[derive(Debug, Clone, Deserialize)]
pub struct MappingConfig {
    /// Axis normalization entries, matched by `hid_axis` name.
    #[serde(default)]
    pub axes: Vec<AxisMapping>,
}

/// Normalization parameters for one HID report axis.
#[derive(Debug, Clone, Deserialize)]
pub struct AxisMapping {
    /// Target HID axis name (e.g., "lx", "ly", "rx", "ry", "lt", "rt").
    pub hid_axis: String,
    /// Minimum raw value from the Deck report (evdev scale).
    pub evdev_min: i32,
    /// Maximum raw value from the Deck report (evdev scale).
    pub evdev_max: i32,
    /// Whether to invert the axis value.
    #[serde(default)]
    pub invert: bool,
    /// Inner deadzone radius (values within this range from center are zeroed).
    #[serde(default)]
    pub deadzone: i32,
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_config() {
        let toml = r#"
[[axes]]
hid_axis = "lx"
evdev_min = -32767
evdev_max = 32767
deadzone = 4000
"#;
        let config = MappingConfig::from_toml(toml).unwrap();
        assert_eq!(config.axes.len(), 1);
        assert_eq!(config.axes[0].deadzone, 4000);
    }

    #[test]
    fn legacy_device_and_button_sections_are_ignored() {
        // Older configs carried [device] and [[buttons]] sections that the
        // reader never consumed; they must still parse (and be ignored).
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

[[buttons]]
evdev_code = 0x130
hid_button = "a"
"#;
        let config = MappingConfig::from_toml(toml).unwrap();
        assert_eq!(config.axes.len(), 1);
    }

    #[test]
    fn reject_unknown_axis() {
        let toml = r#"
[[axes]]
hid_axis = "invalid_axis"
evdev_min = -32767
evdev_max = 32767
"#;
        let err = MappingConfig::from_toml(toml).unwrap_err();
        assert!(err.contains("unknown hid_axis"));
    }

    #[test]
    fn reject_invalid_axis_range() {
        let toml = r#"
[[axes]]
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
        assert_eq!(config.axes.len(), 6); // lx, ly, rx, ry, lt, rt
    }
}
