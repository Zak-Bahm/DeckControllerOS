#![forbid(unsafe_code)]

use crate::mapping::DeviceFilter;
use evdev::{AbsoluteAxisCode, Device, KeyCode};
use std::path::{Path, PathBuf};

/// Metadata about a discovered input device.
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    /// Path to the evdev node (e.g., `/dev/input/event9`).
    pub path: PathBuf,
    /// Device name reported by the kernel.
    pub name: String,
    /// Physical location string.
    pub phys: String,
    /// Vendor ID.
    pub vendor: u16,
    /// Product ID.
    pub product: u16,
    /// Whether this device has absolute axes (sticks/triggers).
    pub has_abs: bool,
    /// Whether this device has gamepad buttons.
    pub has_gamepad_keys: bool,
    /// Whether this device matches the Steam Deck gamepad heuristic.
    pub is_deck_gamepad: bool,
    /// Human-readable summary of capabilities.
    pub caps_summary: String,
}

/// Scan `/dev/input/event*` and return metadata for each device.
///
/// Does not open devices exclusively — this is a read-only enumeration.
pub fn discover_devices() -> Vec<InputDeviceInfo> {
    let mut devices = Vec::new();

    let Ok(entries) = std::fs::read_dir("/dev/input") else {
        return devices;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("event") {
            continue;
        }

        let Ok(dev) = Device::open(&path) else {
            continue;
        };

        let info = build_device_info(&path, &dev);
        devices.push(info);
    }

    // Sort by path for stable output.
    devices.sort_by(|a, b| a.path.cmp(&b.path));
    devices
}

/// Select the first device matching a [`DeviceFilter`].
///
/// Tries in order: exact name match, then vendor/product match,
/// then falls back to the first device flagged `is_deck_gamepad`.
pub fn select_device(
    devices: &[InputDeviceInfo],
    filter: &DeviceFilter,
) -> Option<InputDeviceInfo> {
    // Exact name match
    if let Some(ref name) = filter.name {
        if let Some(d) = devices.iter().find(|d| d.name == *name) {
            return Some(d.clone());
        }
    }
    // VID/PID match with gamepad capabilities
    if let (Some(vid), Some(pid)) = (filter.vendor_id, filter.product_id) {
        if let Some(d) = devices
            .iter()
            .find(|d| d.vendor == vid && d.product == pid && d.has_abs && d.has_gamepad_keys)
        {
            return Some(d.clone());
        }
    }
    // Fallback: first Deck gamepad
    devices.iter().find(|d| d.is_deck_gamepad).cloned()
}

fn build_device_info(path: &Path, dev: &Device) -> InputDeviceInfo {
    let name = dev.name().unwrap_or("").to_string();
    let phys = dev.physical_path().unwrap_or("").to_string();
    let input_id = dev.input_id();
    let vendor = input_id.vendor();
    let product = input_id.product();

    let supported_abs = dev.supported_absolute_axes();
    let has_abs = supported_abs.map_or(false, |axes| {
        axes.contains(AbsoluteAxisCode::ABS_X) || axes.contains(AbsoluteAxisCode::ABS_RX)
    });

    let supported_keys = dev.supported_keys();
    let has_gamepad_keys = supported_keys.map_or(false, |keys| keys.contains(KeyCode::BTN_SOUTH));

    let is_deck_gamepad = is_steam_deck_gamepad(&name, vendor, product, has_abs, has_gamepad_keys);

    let caps_summary = build_caps_summary(dev);

    InputDeviceInfo {
        path: path.to_path_buf(),
        name,
        phys,
        vendor,
        product,
        has_abs,
        has_gamepad_keys,
        is_deck_gamepad,
        caps_summary,
    }
}

fn build_caps_summary(dev: &Device) -> String {
    let mut parts = Vec::new();

    if let Some(keys) = dev.supported_keys() {
        let gamepad_keys: &[(KeyCode, &str)] = &[
            (KeyCode::BTN_SOUTH, "A"),
            (KeyCode::BTN_EAST, "B"),
            (KeyCode::BTN_NORTH, "X"),
            (KeyCode::BTN_WEST, "Y"),
            (KeyCode::BTN_TL, "LB"),
            (KeyCode::BTN_TR, "RB"),
            (KeyCode::BTN_SELECT, "Back"),
            (KeyCode::BTN_START, "Start"),
            (KeyCode::BTN_THUMBL, "LS"),
            (KeyCode::BTN_THUMBR, "RS"),
            (KeyCode::BTN_DPAD_UP, "DU"),
            (KeyCode::BTN_DPAD_DOWN, "DD"),
            (KeyCode::BTN_DPAD_LEFT, "DL"),
            (KeyCode::BTN_DPAD_RIGHT, "DR"),
        ];
        let found: Vec<&str> = gamepad_keys
            .iter()
            .filter(|(code, _)| keys.contains(*code))
            .map(|(_, label)| *label)
            .collect();
        if !found.is_empty() {
            parts.push(format!("buttons=[{}]", found.join(",")));
        }
    }

    if let Some(axes) = dev.supported_absolute_axes() {
        let gamepad_axes: &[(AbsoluteAxisCode, &str)] = &[
            (AbsoluteAxisCode::ABS_X, "LX"),
            (AbsoluteAxisCode::ABS_Y, "LY"),
            (AbsoluteAxisCode::ABS_RX, "RX"),
            (AbsoluteAxisCode::ABS_RY, "RY"),
            (AbsoluteAxisCode::ABS_HAT2Y, "LT"),
            (AbsoluteAxisCode::ABS_HAT2X, "RT"),
        ];
        let found: Vec<&str> = gamepad_axes
            .iter()
            .filter(|(code, _)| axes.contains(*code))
            .map(|(_, label)| *label)
            .collect();
        if !found.is_empty() {
            parts.push(format!("axes=[{}]", found.join(",")));
        }
    }

    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(" ")
    }
}

/// Heuristic to identify the Steam Deck gamepad device.
///
/// The `hid-steam` driver creates multiple input devices for the Deck controller
/// (VID 0x28DE, PID 0x1205). The gamepad is the one named "Steam Deck" with both
/// ABS axes (sticks/triggers) and KEY events (buttons). Other interfaces are
/// keyboard, mouse (trackpad), and motion sensors.
fn is_steam_deck_gamepad(
    name: &str,
    vendor: u16,
    product: u16,
    has_abs: bool,
    has_gamepad_keys: bool,
) -> bool {
    // Primary: exact name match from hid-steam driver
    if name == "Steam Deck" && vendor == 0x28DE && product == 0x1205 {
        return true;
    }
    // Fallback: Valve VID with gamepad capabilities
    if vendor == 0x28DE && has_abs && has_gamepad_keys {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_gamepad_heuristic_exact_match() {
        assert!(is_steam_deck_gamepad(
            "Steam Deck",
            0x28DE,
            0x1205,
            true,
            true
        ));
    }

    #[test]
    fn deck_gamepad_heuristic_rejects_keyboard_interface() {
        // The keyboard interface has keys but no ABS axes
        assert!(!is_steam_deck_gamepad(
            "Valve Software Steam Deck Controller",
            0x28DE,
            0x1205,
            false,
            false,
        ));
    }

    #[test]
    fn deck_gamepad_heuristic_rejects_motion_sensors() {
        // Motion sensors have ABS but no gamepad buttons
        assert!(!is_steam_deck_gamepad(
            "Steam Deck Motion Sensors",
            0x28DE,
            0x1205,
            true,
            false,
        ));
    }

    #[test]
    fn deck_gamepad_fallback_valve_vid_with_gamepad_caps() {
        assert!(is_steam_deck_gamepad(
            "Unknown Valve Device",
            0x28DE,
            0x9999,
            true,
            true
        ));
    }

    #[test]
    fn non_valve_device_rejected() {
        assert!(!is_steam_deck_gamepad(
            "Generic Gamepad",
            0x1234,
            0x5678,
            true,
            true
        ));
    }

    fn make_test_info(name: &str, vendor: u16, product: u16, deck: bool) -> InputDeviceInfo {
        InputDeviceInfo {
            path: PathBuf::from(format!("/dev/input/event{}", vendor)),
            name: name.to_string(),
            phys: String::new(),
            vendor,
            product,
            has_abs: deck,
            has_gamepad_keys: deck,
            is_deck_gamepad: deck,
            caps_summary: String::new(),
        }
    }

    #[test]
    fn select_device_by_name() {
        let devices = vec![
            make_test_info("Keyboard", 0x1234, 0x0001, false),
            make_test_info("Steam Deck", 0x28DE, 0x1205, true),
        ];
        let filter = DeviceFilter {
            name: Some("Steam Deck".to_string()),
            vendor_id: None,
            product_id: None,
        };
        let selected = select_device(&devices, &filter).unwrap();
        assert_eq!(selected.name, "Steam Deck");
    }

    #[test]
    fn select_device_fallback_to_deck_gamepad() {
        let devices = vec![
            make_test_info("Keyboard", 0x1234, 0x0001, false),
            make_test_info("Steam Deck", 0x28DE, 0x1205, true),
        ];
        let filter = DeviceFilter {
            name: None,
            vendor_id: None,
            product_id: None,
        };
        let selected = select_device(&devices, &filter).unwrap();
        assert_eq!(selected.name, "Steam Deck");
    }

    #[test]
    fn select_device_returns_none_when_no_match() {
        let devices = vec![make_test_info("Keyboard", 0x1234, 0x0001, false)];
        let filter = DeviceFilter {
            name: Some("Steam Deck".to_string()),
            vendor_id: None,
            product_id: None,
        };
        assert!(select_device(&devices, &filter).is_none());
    }
}
