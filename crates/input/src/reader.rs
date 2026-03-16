use crate::hidraw::{self, HidrawDevice, DECK_REPORT_TYPE, REPORT_SIZE};
use crate::mapping::{AxisMapping, MappingConfig};
use common::hid::{
    InputReport, XBOX_BUTTON_A, XBOX_BUTTON_B, XBOX_BUTTON_HOME, XBOX_BUTTON_LB, XBOX_BUTTON_LS,
    XBOX_BUTTON_RB, XBOX_BUTTON_RS, XBOX_BUTTON_SELECT, XBOX_BUTTON_START, XBOX_BUTTON_X,
    XBOX_BUTTON_Y, XBOX_STICK_MAX, XBOX_STICK_MIN, XBOX_TRIGGER_MAX, XBOX_TRIGGER_MIN,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Reads raw HID reports from the Steam Deck controller via hidraw and
/// maintains the current mapped gamepad state as an Xbox-style `InputReport`.
pub struct InputReader {
    state: Arc<Mutex<InputReport>>,
    running: Arc<AtomicBool>,
    _thread: thread::JoinHandle<()>,
}

/// Axis normalization config extracted from MappingConfig, keyed by axis name.
struct AxisConfig {
    lx: AxisMapping,
    ly: AxisMapping,
    rx: AxisMapping,
    ry: AxisMapping,
    lt: AxisMapping,
    rt: AxisMapping,
}

impl InputReader {
    /// Create a new reader that discovers the Deck's hidraw device, disables
    /// lizard mode, and starts reading raw input reports on a background thread.
    pub fn new(config: MappingConfig) -> Result<Self, String> {
        let path = hidraw::discover_deck_hidraw()?;
        let dev = HidrawDevice::open(&path)?;

        eprintln!("input: opened hidraw {}", path.display());

        dev.disable_lizard_mode()?;
        eprintln!("input: lizard mode disabled");

        let axis_config = build_axis_config(&config)?;

        let state = Arc::new(Mutex::new(InputReport::default()));
        let running = Arc::new(AtomicBool::new(true));

        let thread_state = Arc::clone(&state);
        let thread_running = Arc::clone(&running);
        let handle = thread::spawn(move || {
            hidraw_loop(dev, axis_config, thread_state, thread_running);
        });

        Ok(Self {
            state,
            running,
            _thread: handle,
        })
    }

    /// Returns the latest mapped input state as an HID `InputReport`.
    pub fn current_report(&self) -> InputReport {
        *self.state.lock().unwrap()
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn build_axis_config(config: &MappingConfig) -> Result<AxisConfig, String> {
    let find = |name: &str| -> Result<AxisMapping, String> {
        config
            .axes
            .iter()
            .find(|a| a.hid_axis == name)
            .cloned()
            .ok_or_else(|| format!("missing axis mapping for '{name}'"))
    };
    Ok(AxisConfig {
        lx: find("lx")?,
        ly: find("ly")?,
        rx: find("rx")?,
        ry: find("ry")?,
        lt: find("lt")?,
        rt: find("rt")?,
    })
}

fn hidraw_loop(
    mut dev: HidrawDevice,
    axis_config: AxisConfig,
    state: Arc<Mutex<InputReport>>,
    running: Arc<AtomicBool>,
) {
    let mut buf = [0u8; REPORT_SIZE];

    while running.load(Ordering::Relaxed) {
        match dev.read_report_timeout(&mut buf, 100) {
            Ok(0) => continue, // timeout, check running flag
            Ok(n) if n >= 56 => {
                // Validate report header: data[0]=0x01, data[1]=0x00, data[2]=type
                if buf[0] == 0x01 && buf[1] == 0x00 && buf[2] == DECK_REPORT_TYPE {
                    let report = parse_deck_report(&buf, &axis_config);
                    *state.lock().unwrap() = report;
                }
            }
            Ok(_) => {} // short read, ignore
            Err(e) => {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                eprintln!("input: hidraw read error: {e}");
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

/// Parse a raw 64-byte Deck HID report (type 0x09) into an Xbox InputReport.
///
/// Byte layout from `steam_do_deck_input_event` in hid-steam.c:
///
/// Buttons (bit positions):
///   data[8]:  A(7) X(6) B(5) Y(4) LB(3) RB(2)
///   data[9]:  DPAD_UP(0) DPAD_RIGHT(1) DPAD_LEFT(2) DPAD_DOWN(3)
///             SELECT(4) HOME(5) START(6)
///   data[10]: LS(6)
///   data[11]: RS(2)
///
/// Axes (little-endian i16):
///   data[48..50]: left stick X
///   data[50..52]: left stick Y (raw Y-up positive, negate for standard)
///   data[52..54]: right stick X
///   data[54..56]: right stick Y (raw Y-up positive, negate for standard)
///   data[44..46]: left trigger  (0..32767)
///   data[46..48]: right trigger (0..32767)
fn parse_deck_report(data: &[u8; REPORT_SIZE], config: &AxisConfig) -> InputReport {
    let mut report = InputReport::default();

    // --- Axes ---
    let raw_lx = i16::from_le_bytes([data[48], data[49]]) as i32;
    let raw_ly = -(i16::from_le_bytes([data[50], data[51]]) as i32);
    let raw_rx = i16::from_le_bytes([data[52], data[53]]) as i32;
    let raw_ry = -(i16::from_le_bytes([data[54], data[55]]) as i32);
    let raw_lt = i16::from_le_bytes([data[44], data[45]]) as i32;
    let raw_rt = i16::from_le_bytes([data[46], data[47]]) as i32;

    report.lx = normalize_stick(raw_lx, &config.lx);
    report.ly = normalize_stick(raw_ly, &config.ly);
    report.rx = normalize_stick(raw_rx, &config.rx);
    report.ry = normalize_stick(raw_ry, &config.ry);
    report.lt = normalize_trigger(raw_lt, &config.lt);
    report.rt = normalize_trigger(raw_rt, &config.rt);

    // --- Buttons ---
    let b8 = data[8];
    let b9 = data[9];
    let b10 = data[10];
    let b11 = data[11];

    if b8 & (1 << 7) != 0 {
        report.buttons |= XBOX_BUTTON_A;
    }
    if b8 & (1 << 5) != 0 {
        report.buttons |= XBOX_BUTTON_B;
    }
    if b8 & (1 << 6) != 0 {
        report.buttons |= XBOX_BUTTON_X;
    }
    if b8 & (1 << 4) != 0 {
        report.buttons |= XBOX_BUTTON_Y;
    }
    if b8 & (1 << 3) != 0 {
        report.buttons |= XBOX_BUTTON_LB;
    }
    if b8 & (1 << 2) != 0 {
        report.buttons |= XBOX_BUTTON_RB;
    }
    if b9 & (1 << 4) != 0 {
        report.buttons |= XBOX_BUTTON_SELECT;
    }
    if b9 & (1 << 6) != 0 {
        report.buttons |= XBOX_BUTTON_START;
    }
    if b9 & (1 << 5) != 0 {
        report.buttons |= XBOX_BUTTON_HOME;
    }
    if b10 & (1 << 6) != 0 {
        report.buttons |= XBOX_BUTTON_LS;
    }
    if b11 & (1 << 2) != 0 {
        report.buttons |= XBOX_BUTTON_RS;
    }

    // --- D-pad → hat switch ---
    let dpad = [
        b9 & (1 << 0) != 0, // up
        b9 & (1 << 3) != 0, // down
        b9 & (1 << 2) != 0, // left
        b9 & (1 << 1) != 0, // right
    ];
    report.hat = dpad_to_hat(dpad);

    report
}

/// Normalize a stick axis value to the HID i16 range (-32768..32767).
/// Applies deadzone: values within the deadzone radius from center are zeroed.
fn normalize_stick(raw: i32, mapping: &AxisMapping) -> i16 {
    let center = (i64::from(mapping.evdev_min) + i64::from(mapping.evdev_max)) / 2;
    let mut val = i64::from(raw) - center;

    if mapping.invert {
        val = -val;
    }

    let dz = i64::from(mapping.deadzone);
    if val.abs() <= dz {
        return 0;
    }

    let half_range = (i64::from(mapping.evdev_max) - i64::from(mapping.evdev_min)) / 2;
    let effective_range = half_range - dz;
    if effective_range <= 0 {
        return 0;
    }

    let sign = val.signum();
    val = val.abs() - dz;

    let out_range = i64::from(XBOX_STICK_MAX);
    let scaled = (val * out_range) / effective_range;
    (sign * scaled).clamp(i64::from(XBOX_STICK_MIN), i64::from(XBOX_STICK_MAX)) as i16
}

/// Normalize a trigger axis value to the HID u16 range (0..1023).
fn normalize_trigger(raw: i32, mapping: &AxisMapping) -> u16 {
    let in_range = i64::from(mapping.evdev_max) - i64::from(mapping.evdev_min);
    if in_range <= 0 {
        return XBOX_TRIGGER_MIN;
    }

    let val = i64::from(raw) - i64::from(mapping.evdev_min);
    let out_range = i64::from(XBOX_TRIGGER_MAX);
    let scaled = (val * out_range) / in_range;
    scaled.clamp(i64::from(XBOX_TRIGGER_MIN), i64::from(XBOX_TRIGGER_MAX)) as u16
}

/// Convert d-pad button state [up, down, left, right] to Xbox hat switch value.
/// Hat values: 0=none, 1=N, 2=NE, 3=E, 4=SE, 5=S, 6=SW, 7=W, 8=NW.
fn dpad_to_hat(dpad: [bool; 4]) -> u8 {
    let [up, down, left, right] = dpad;
    match (up, down, left, right) {
        (true, false, false, false) => 1, // N
        (true, false, false, true) => 2,  // NE
        (false, false, false, true) => 3, // E
        (false, true, false, true) => 4,  // SE
        (false, true, false, false) => 5, // S
        (false, true, true, false) => 6,  // SW
        (false, false, true, false) => 7, // W
        (true, false, true, false) => 8,  // NW
        _ => 0,                           // None / conflicting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stick_mapping(deadzone: i32) -> AxisMapping {
        AxisMapping {
            evdev_code: 0,
            hid_axis: "lx".to_string(),
            evdev_min: -32767,
            evdev_max: 32767,
            invert: false,
            deadzone,
        }
    }

    fn trigger_mapping() -> AxisMapping {
        AxisMapping {
            evdev_code: 0,
            hid_axis: "lt".to_string(),
            evdev_min: 0,
            evdev_max: 32767,
            invert: false,
            deadzone: 0,
        }
    }

    #[test]
    fn stick_center_is_zero() {
        assert_eq!(normalize_stick(0, &stick_mapping(0)), 0);
    }

    #[test]
    fn stick_full_deflection() {
        assert_eq!(normalize_stick(32767, &stick_mapping(0)), XBOX_STICK_MAX);
        assert_eq!(
            normalize_stick(-32767, &stick_mapping(0)),
            XBOX_STICK_MIN + 1
        );
    }

    #[test]
    fn stick_deadzone_zeroes_small_values() {
        let m = stick_mapping(4000);
        assert_eq!(normalize_stick(3999, &m), 0);
        assert_eq!(normalize_stick(-3999, &m), 0);
        assert_eq!(normalize_stick(0, &m), 0);
        assert_ne!(normalize_stick(4001, &m), 0);
    }

    #[test]
    fn stick_deadzone_scales_remaining_range() {
        let m = stick_mapping(4000);
        assert_eq!(normalize_stick(32767, &m), XBOX_STICK_MAX);
        assert_eq!(normalize_stick(-32767, &m), XBOX_STICK_MIN + 1);
    }

    #[test]
    fn stick_invert() {
        let mut m = stick_mapping(0);
        m.invert = true;
        assert!(normalize_stick(32767, &m) < 0);
        assert!(normalize_stick(-32767, &m) > 0);
    }

    #[test]
    fn trigger_zero_at_min() {
        assert_eq!(normalize_trigger(0, &trigger_mapping()), XBOX_TRIGGER_MIN);
    }

    #[test]
    fn trigger_max_at_max() {
        assert_eq!(
            normalize_trigger(32767, &trigger_mapping()),
            XBOX_TRIGGER_MAX
        );
    }

    #[test]
    fn trigger_midpoint() {
        let val = normalize_trigger(16383, &trigger_mapping());
        assert!(val > 500 && val < 520, "midpoint trigger was {val}");
    }

    #[test]
    fn dpad_cardinal_directions() {
        assert_eq!(dpad_to_hat([true, false, false, false]), 1); // N
        assert_eq!(dpad_to_hat([false, true, false, false]), 5); // S
        assert_eq!(dpad_to_hat([false, false, true, false]), 7); // W
        assert_eq!(dpad_to_hat([false, false, false, true]), 3); // E
    }

    #[test]
    fn dpad_diagonals() {
        assert_eq!(dpad_to_hat([true, false, false, true]), 2); // NE
        assert_eq!(dpad_to_hat([false, true, false, true]), 4); // SE
        assert_eq!(dpad_to_hat([false, true, true, false]), 6); // SW
        assert_eq!(dpad_to_hat([true, false, true, false]), 8); // NW
    }

    #[test]
    fn dpad_none() {
        assert_eq!(dpad_to_hat([false, false, false, false]), 0);
    }

    #[test]
    fn dpad_conflicting_returns_none() {
        assert_eq!(dpad_to_hat([true, true, false, false]), 0); // up+down
    }

    #[test]
    fn parse_report_buttons() {
        let config = test_axis_config();
        let mut data = [0u8; REPORT_SIZE];
        data[0] = 0x01;
        data[1] = 0x00;
        data[2] = DECK_REPORT_TYPE;

        // Press A (b8 bit 7) and Start (b9 bit 6)
        data[8] = 1 << 7;
        data[9] = 1 << 6;

        let report = parse_deck_report(&data, &config);
        assert_eq!(report.buttons & XBOX_BUTTON_A, XBOX_BUTTON_A);
        assert_eq!(report.buttons & XBOX_BUTTON_START, XBOX_BUTTON_START);
        assert_eq!(report.buttons & XBOX_BUTTON_B, 0);
    }

    #[test]
    fn parse_report_dpad() {
        let config = test_axis_config();
        let mut data = [0u8; REPORT_SIZE];
        data[0] = 0x01;
        data[1] = 0x00;
        data[2] = DECK_REPORT_TYPE;

        // D-pad up (b9 bit 0) + right (b9 bit 1) = NE
        data[9] = (1 << 0) | (1 << 1);

        let report = parse_deck_report(&data, &config);
        assert_eq!(report.hat, 2); // NE
    }

    #[test]
    fn parse_report_axes() {
        let config = test_axis_config();
        let mut data = [0u8; REPORT_SIZE];
        data[0] = 0x01;
        data[1] = 0x00;
        data[2] = DECK_REPORT_TYPE;

        // Left stick X = 16000 (little-endian at offset 48)
        let lx_bytes = 16000_i16.to_le_bytes();
        data[48] = lx_bytes[0];
        data[49] = lx_bytes[1];

        // Left trigger = 32767 (little-endian at offset 44)
        let lt_bytes = 32767_i16.to_le_bytes();
        data[44] = lt_bytes[0];
        data[45] = lt_bytes[1];

        let report = parse_deck_report(&data, &config);
        assert!(report.lx > 0, "lx should be positive, got {}", report.lx);
        assert_eq!(report.lt, XBOX_TRIGGER_MAX);
    }

    #[test]
    fn parse_report_y_axis_negated() {
        let config = test_axis_config();
        let mut data = [0u8; REPORT_SIZE];
        data[0] = 0x01;
        data[1] = 0x00;
        data[2] = DECK_REPORT_TYPE;

        // Left stick Y raw = +16000 (stick pushed up)
        // After negation should become -16000 (standard Y-down positive)
        let ly_bytes = 16000_i16.to_le_bytes();
        data[50] = ly_bytes[0];
        data[51] = ly_bytes[1];

        let report = parse_deck_report(&data, &config);
        assert!(
            report.ly < 0,
            "ly should be negative (Y negated), got {}",
            report.ly
        );
    }

    fn test_axis_config() -> AxisConfig {
        AxisConfig {
            lx: stick_mapping(4000),
            ly: stick_mapping(4000),
            rx: stick_mapping(4000),
            ry: stick_mapping(4000),
            lt: trigger_mapping(),
            rt: trigger_mapping(),
        }
    }
}
