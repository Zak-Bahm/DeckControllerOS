//! Battery capacity readout via sysfs.
//!
//! Discovers the first `/sys/class/power_supply/*` entry whose `type` is
//! `Battery` and reads its `capacity` (0–100) on demand. If no battery is
//! present (e.g. dev VM), capacity is unavailable and callers keep the
//! default 100% report.

use std::fs;
use std::path::PathBuf;

const POWER_SUPPLY_ROOT: &str = "/sys/class/power_supply";

pub struct BatterySource {
    capacity_path: Option<PathBuf>,
}

impl BatterySource {
    pub fn discover() -> Self {
        Self::discover_in(POWER_SUPPLY_ROOT)
    }

    fn discover_in(root: &str) -> Self {
        let mut capacity_path = None;
        if let Ok(entries) = fs::read_dir(root) {
            let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            dirs.sort();
            for dir in dirs {
                let is_battery = fs::read_to_string(dir.join("type"))
                    .map(|t| t.trim() == "Battery")
                    .unwrap_or(false);
                if is_battery && dir.join("capacity").is_file() {
                    eprintln!("hidd: battery source: {}", dir.display());
                    capacity_path = Some(dir.join("capacity"));
                    break;
                }
            }
        }
        if capacity_path.is_none() {
            eprintln!("hidd: no battery under {root}; reporting 100%");
        }
        Self { capacity_path }
    }

    /// Current capacity 0..=100, or `None` if no battery / unreadable.
    pub fn capacity(&self) -> Option<u8> {
        let path = self.capacity_path.as_ref()?;
        parse_capacity(&fs::read_to_string(path).ok()?)
    }
}

fn parse_capacity(raw: &str) -> Option<u8> {
    raw.trim().parse::<u8>().ok().map(|v| v.min(100))
}

/// Scale a 0..=100 capacity to the HID descriptor's Battery Strength
/// logical range 0..=255 (input report 0x04).
pub fn capacity_to_battery_strength(capacity: u8) -> u8 {
    (u16::from(capacity.min(100)) * 255 / 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_capacity_handles_sysfs_newline() {
        assert_eq!(parse_capacity("87\n"), Some(87));
        assert_eq!(parse_capacity("0"), Some(0));
        assert_eq!(parse_capacity("100\n"), Some(100));
    }

    #[test]
    fn parse_capacity_clamps_and_rejects_garbage() {
        assert_eq!(parse_capacity("150"), Some(100));
        assert_eq!(parse_capacity("abc"), None);
        assert_eq!(parse_capacity(""), None);
        assert_eq!(parse_capacity("-5"), None);
    }

    #[test]
    fn battery_strength_scales_full_range() {
        assert_eq!(capacity_to_battery_strength(0), 0);
        assert_eq!(capacity_to_battery_strength(50), 127);
        assert_eq!(capacity_to_battery_strength(100), 255);
        // Out-of-range capacity is clamped, not wrapped.
        assert_eq!(capacity_to_battery_strength(200), 255);
    }
}
