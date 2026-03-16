//! Low-level hidraw interface for the Steam Deck controller.
//!
//! This module discovers the Deck's client hidraw device, sends HID feature
//! reports to disable lizard mode, and reads raw 64-byte input reports.
//!
//! Safety: ioctl and poll calls require unsafe blocks. All unsafe usage is
//! confined to `send_feature_report` and `read_report_timeout`.

use std::fs;
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

const STEAM_VID: u16 = 0x28DE;
const STEAM_DECK_PID: u16 = 0x1205;

// HID command IDs from hid-steam.c
const ID_CLEAR_DIGITAL_MAPPINGS: u8 = 0x81;
const ID_SET_SETTINGS_VALUES: u8 = 0x87;

// Setting register IDs (sequential enum starting at 0 in kernel)
const SETTING_LEFT_TRACKPAD_MODE: u8 = 7;
const SETTING_RIGHT_TRACKPAD_MODE: u8 = 8;
const SETTING_LEFT_TRACKPAD_CLICK_PRESSURE: u8 = 52; // 0x34
const SETTING_RIGHT_TRACKPAD_CLICK_PRESSURE: u8 = 53; // 0x35
const SETTING_STEAM_WATCHDOG_ENABLE: u8 = 71; // 0x47

// Trackpad mode values (sequential enum starting at 0 in kernel)
const TRACKPAD_NONE: u16 = 7;

/// Deck input report type byte (data[2]).
pub const DECK_REPORT_TYPE: u8 = 0x09;

/// Expected report size from the controller.
pub const REPORT_SIZE: usize = 64;

/// Compute HIDIOCSFEATURE ioctl number for a given buffer length.
/// HIDIOCSFEATURE(len) = _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x06, len)
fn hidiocsfeature(len: usize) -> libc::c_ulong {
    let dir: u64 = 3; // _IOC_WRITE | _IOC_READ
    let ty: u64 = b'H' as u64;
    let nr: u64 = 0x06;
    let size: u64 = len as u64;
    ((dir << 30) | (size << 16) | (ty << 8) | nr) as libc::c_ulong
}

pub struct HidrawDevice {
    file: fs::File,
}

/// Find the Steam Deck's client hidraw device.
///
/// The hid-steam driver creates multiple HID sub-devices for the Deck controller
/// (VID 0x28DE, PID 0x1205). The client device is the one without any input
/// subdirectory in sysfs — it only provides a hidraw interface. Opening it
/// triggers `steam_client_ll_open()` in the kernel which enables raw data
/// forwarding.
pub fn discover_deck_hidraw() -> Result<PathBuf, String> {
    let entries = fs::read_dir("/sys/class/hidraw")
        .map_err(|e| format!("cannot read /sys/class/hidraw: {e}"))?;

    let mut candidates: Vec<(PathBuf, bool)> = Vec::new(); // (dev_path, has_input)

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("hidraw") {
            continue;
        }

        let uevent_path = entry.path().join("device/uevent");
        let Ok(uevent) = fs::read_to_string(&uevent_path) else {
            continue;
        };

        // Parse HID_ID=BBBB:VVVVVVVV:PPPPPPPP
        let Some(hid_id_line) = uevent.lines().find(|l| l.starts_with("HID_ID=")) else {
            continue;
        };
        let parts: Vec<&str> = hid_id_line["HID_ID=".len()..].split(':').collect();
        if parts.len() != 3 {
            continue;
        }
        let vid = u32::from_str_radix(parts[1], 16).unwrap_or(0) as u16;
        let pid = u32::from_str_radix(parts[2], 16).unwrap_or(0) as u16;
        if vid != STEAM_VID || pid != STEAM_DECK_PID {
            continue;
        }

        // Check whether this device has input sub-devices (evdev nodes).
        // The client device does NOT have any.
        let has_input = entry.path().join("device/input").is_dir();
        let dev_path = PathBuf::from(format!("/dev/{name}"));
        candidates.push((dev_path, has_input));
    }

    if candidates.is_empty() {
        return Err("no Steam Deck hidraw device found".to_string());
    }

    // Sort by path for deterministic ordering.
    candidates.sort_by(|a, b| a.0.cmp(&b.0));

    // Prefer the device without input subdirectory (the client device).
    if let Some((path, _)) = candidates.iter().find(|(_, has_input)| !has_input) {
        return Ok(path.clone());
    }

    // Fallback: take the last (highest-numbered) device.
    Ok(candidates.last().unwrap().0.clone())
}

impl HidrawDevice {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = fs::File::options()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        Ok(Self { file })
    }

    /// Send a HID feature report (SET_REPORT) to the controller.
    ///
    /// The buffer is prepended with report ID 0x00 and padded to 65 bytes
    /// (1 byte report ID + 64 bytes payload), matching the kernel's
    /// `steam_send_report` format.
    fn send_feature_report(&self, data: &[u8]) -> Result<(), String> {
        let mut buf = [0u8; 65]; // report ID (0) + 64 bytes payload
        let len = data.len().min(64);
        buf[1..1 + len].copy_from_slice(&data[..len]);

        let fd = self.file.as_raw_fd();
        // SAFETY: ioctl with HIDIOCSFEATURE sends a feature report via the
        // hidraw device. The buffer is stack-allocated and correctly sized.
        let ret = unsafe { libc::ioctl(fd, hidiocsfeature(buf.len()), buf.as_mut_ptr()) };
        if ret < 0 {
            return Err(format!(
                "HIDIOCSFEATURE failed: {}",
                io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    /// Disable lizard mode on the Steam Deck controller.
    ///
    /// Sends two HID feature reports matching the kernel's
    /// `steam_set_lizard_mode(false)` for Deck:
    /// 1. Clear digital mappings (stops keyboard/mouse emulation)
    /// 2. Set trackpad modes to NONE, max click pressure, disable watchdog
    pub fn disable_lizard_mode(&self) -> Result<(), String> {
        // Step 1: Clear digital mappings
        self.send_feature_report(&[ID_CLEAR_DIGITAL_MAPPINGS])?;

        // Step 2: Write settings
        // Format: 0x87 <len> (reg val_lo val_hi)*
        let settings: &[(u8, u16)] = &[
            (SETTING_LEFT_TRACKPAD_MODE, TRACKPAD_NONE),
            (SETTING_RIGHT_TRACKPAD_MODE, TRACKPAD_NONE),
            (SETTING_LEFT_TRACKPAD_CLICK_PRESSURE, 0xFFFF),
            (SETTING_RIGHT_TRACKPAD_CLICK_PRESSURE, 0xFFFF),
            (SETTING_STEAM_WATCHDOG_ENABLE, 0),
        ];

        let payload_len = settings.len() * 3;
        let mut cmd = vec![0u8; 2 + payload_len];
        cmd[0] = ID_SET_SETTINGS_VALUES;
        cmd[1] = payload_len as u8;
        for (i, &(reg, val)) in settings.iter().enumerate() {
            let off = 2 + i * 3;
            cmd[off] = reg;
            cmd[off + 1] = (val & 0xFF) as u8;
            cmd[off + 2] = (val >> 8) as u8;
        }

        self.send_feature_report(&cmd)?;
        Ok(())
    }

    /// Read a raw HID report with a timeout.
    ///
    /// Returns the number of bytes read, or 0 on timeout.
    pub fn read_report_timeout(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, io::Error> {
        let fd = self.file.as_raw_fd();
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };

        // SAFETY: poll with a single fd and bounded timeout. The pollfd struct
        // is stack-allocated and valid for the duration of the call.
        let ret = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        if ret == 0 {
            return Ok(0); // timeout
        }

        self.file.read(buf)
    }
}
