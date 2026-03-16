#![forbid(unsafe_code)]

use std::env;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use common::config::{AxisName, HidConfig, PatternConfig, DEFAULT_HID_CONFIG_PATH};
use common::hid::{InputReport, OutputReport};

mod hog;
use hog::HogRuntime;

const DEV_UHID: &str = "/dev/uhid";
const UHID_DESTROY: u32 = 1;
const UHID_START: u32 = 2;
const UHID_STOP: u32 = 3;
const UHID_OPEN: u32 = 4;
const UHID_CLOSE: u32 = 5;
const UHID_OUTPUT: u32 = 6;
const UHID_GET_REPORT: u32 = 9;
const UHID_GET_REPORT_REPLY: u32 = 10;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;
const UHID_SET_REPORT: u32 = 13;
const UHID_SET_REPORT_REPLY: u32 = 14;

const UHID_EVENT_SIZE: usize = 4376;
const UHID_OUTPUT_DATA_MAX: usize = 4096;
const UHID_ERR_NOT_SUPPORTED: u16 = 95;

const BUS_BLUETOOTH: u16 = 0x05;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hidd: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<()> {
    let mut validate_config = false;
    let mut self_test = false;
    let mut config_path = DEFAULT_HID_CONFIG_PATH.to_string();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--validate-config" => validate_config = true,
            "--self-test" => self_test = true,
            "--config" => {
                config_path = args
                    .next()
                    .ok_or_else(|| anyhow!("missing value for --config"))?;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => {
                return Err(anyhow!("unknown argument: {other}"));
            }
        }
    }

    let cfg = HidConfig::load_from_path(&config_path)?;
    if validate_config {
        println!(
            "HID config OK: name=\"{}\" profile={} vid=0x{:04x} pid=0x{:04x} version=0x{:04x} rate={}Hz",
            cfg.device.name,
            cfg.profile.mode.as_str(),
            cfg.profile.vendor_id,
            cfg.profile.product_id,
            cfg.profile.version,
            cfg.report.rate_hz
        );
        return Ok(());
    }

    if self_test {
        run_self_test(&cfg)?;
        return Ok(());
    }

    run_daemon(&cfg)
}

fn print_help() {
    println!("Usage:");
    println!("  hidd --validate-config [--config <path>]");
    println!("  hidd --self-test [--config <path>]");
    println!("  hidd [--config <path>]");
    println!("Defaults:");
    println!("  --config {}", DEFAULT_HID_CONFIG_PATH);
}

fn run_self_test(cfg: &HidConfig) -> Result<()> {
    let mut uhid = UhidDevice::open()?;
    uhid.create(cfg)?;
    uhid.destroy()?;
    println!("UHID self-test OK");
    Ok(())
}

fn run_daemon(cfg: &HidConfig) -> Result<()> {
    let mut uhid = UhidDevice::open()?;
    uhid.create(cfg)?;
    uhid.start_event_drain()?;
    let hog = HogRuntime::register(cfg)?;

    println!(
        "hidd started: name=\"{}\" profile={} vid=0x{:04x} pid=0x{:04x} version=0x{:04x} rate={}Hz",
        cfg.device.name,
        cfg.profile.mode.as_str(),
        cfg.profile.vendor_id,
        cfg.profile.product_id,
        cfg.profile.version,
        cfg.report.rate_hz
    );
    println!("hidd BLE HOGP registered: adapter={}", hog.adapter_path());

    let period = Duration::from_nanos(1_000_000_000u64 / u64::from(cfg.report.rate_hz));
    let mut pattern = PatternState::new(&cfg.pattern);
    let mut next_tick = Instant::now();

    loop {
        let report = pattern.next_report();
        let report_bytes = report.to_bytes();
        uhid.send_input_report(&report_bytes)?;
        hog.publish_input_report(&report_bytes)?;
        next_tick += period;

        let now = Instant::now();
        if next_tick > now {
            thread::sleep(next_tick - now);
        } else {
            next_tick = now;
        }
    }
}

struct UhidDevice {
    file: std::fs::File,
    created: bool,
}

impl UhidDevice {
    fn open() -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DEV_UHID)
            .map_err(|e| anyhow!("failed to open {DEV_UHID}: {e}"))?;
        Ok(Self {
            file,
            created: false,
        })
    }

    fn create(&mut self, cfg: &HidConfig) -> Result<()> {
        let event = build_create2_event(cfg);
        self.file
            .write_all(&event)
            .map_err(|e| anyhow!("failed to write UHID_CREATE2 event: {e}"))?;
        self.created = true;
        Ok(())
    }

    fn start_event_drain(&self) -> Result<()> {
        let mut io = self
            .file
            .try_clone()
            .map_err(|e| anyhow!("failed to clone UHID fd for event drain: {e}"))?;

        thread::Builder::new()
            .name("hidd-uhid-events".to_string())
            .spawn(move || drain_uhid_events(&mut io))
            .map_err(|e| anyhow!("failed to spawn UHID event drain thread: {e}"))?;

        Ok(())
    }

    fn send_input_report(&mut self, report: &[u8]) -> Result<()> {
        if report.len() > UHID_OUTPUT_DATA_MAX {
            return Err(anyhow!(
                "report too large for UHID_INPUT2: {}",
                report.len()
            ));
        }

        let mut event = Vec::with_capacity(6 + report.len());
        event.extend_from_slice(&UHID_INPUT2.to_ne_bytes());
        event.extend_from_slice(&(report.len() as u16).to_ne_bytes());
        event.extend_from_slice(report);

        self.file
            .write_all(&event)
            .map_err(|e| anyhow!("failed to write UHID_INPUT2 report: {e}"))?;
        Ok(())
    }

    fn destroy(&mut self) -> Result<()> {
        if !self.created {
            return Ok(());
        }
        self.file
            .write_all(&UHID_DESTROY.to_ne_bytes())
            .map_err(|e| anyhow!("failed to write UHID_DESTROY event: {e}"))?;
        self.created = false;
        Ok(())
    }
}

impl Drop for UhidDevice {
    fn drop(&mut self) {
        let _ = self.destroy();
    }
}

fn write_padded(dst: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    let len = bytes.len().min(dst.len().saturating_sub(1));
    dst[..len].copy_from_slice(&bytes[..len]);
}

fn build_create2_event(cfg: &HidConfig) -> Vec<u8> {
    // UHID_CREATE2 payload layout from linux/uapi/linux/uhid.h.
    const CREATE2_PAYLOAD_LEN: usize = 4372;
    const OFF_NAME: usize = 0;
    const OFF_PHYS: usize = 128;
    const OFF_UNIQ: usize = 192;
    const OFF_RD_SIZE: usize = 256;
    const OFF_BUS: usize = 258;
    const OFF_VENDOR: usize = 260;
    const OFF_PRODUCT: usize = 264;
    const OFF_VERSION: usize = 268;
    const OFF_COUNTRY: usize = 272;
    const OFF_RD_DATA: usize = 276;

    let descriptor = cfg.profile.mode.report_descriptor();

    let mut payload = vec![0u8; CREATE2_PAYLOAD_LEN];
    write_padded(&mut payload[OFF_NAME..OFF_PHYS], &cfg.device.name);
    write_padded(&mut payload[OFF_PHYS..OFF_UNIQ], "bluetooth");
    write_padded(
        &mut payload[OFF_UNIQ..OFF_RD_SIZE],
        cfg.profile.mode.as_str(),
    );

    payload[OFF_RD_SIZE..OFF_RD_SIZE + 2].copy_from_slice(&(descriptor.len() as u16).to_ne_bytes());
    payload[OFF_BUS..OFF_BUS + 2].copy_from_slice(&BUS_BLUETOOTH.to_ne_bytes());
    payload[OFF_VENDOR..OFF_VENDOR + 4]
        .copy_from_slice(&(u32::from(cfg.profile.vendor_id)).to_ne_bytes());
    payload[OFF_PRODUCT..OFF_PRODUCT + 4]
        .copy_from_slice(&(u32::from(cfg.profile.product_id)).to_ne_bytes());
    payload[OFF_VERSION..OFF_VERSION + 4]
        .copy_from_slice(&(u32::from(cfg.profile.version)).to_ne_bytes());
    payload[OFF_COUNTRY..OFF_COUNTRY + 4]
        .copy_from_slice(&(u32::from(cfg.profile.country)).to_ne_bytes());
    payload[OFF_RD_DATA..OFF_RD_DATA + descriptor.len()].copy_from_slice(descriptor);

    let mut event = Vec::with_capacity(4 + payload.len());
    event.extend_from_slice(&UHID_CREATE2.to_ne_bytes());
    event.extend_from_slice(&payload);
    event
}

fn drain_uhid_events(io: &mut std::fs::File) {
    let mut event = [0u8; UHID_EVENT_SIZE];
    let mut dropped_output_reports = 0u64;

    loop {
        match io.read_exact(&mut event) {
            Ok(()) => {
                if let Err(err) = handle_uhid_event(io, &event, &mut dropped_output_reports) {
                    eprintln!("hidd: failed to handle UHID event: {err}");
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => {
                eprintln!("hidd: UHID event drain stopped: {err}");
                return;
            }
        }
    }
}

fn handle_uhid_event(
    io: &mut std::fs::File,
    event: &[u8],
    dropped_output_reports: &mut u64,
) -> Result<()> {
    let event_type = read_u32(event, 0).ok_or_else(|| anyhow!("short UHID event"))?;

    match event_type {
        UHID_START => {
            if let Some(flags) = read_u64(event, 4) {
                eprintln!("hidd: UHID_START flags=0x{flags:016x}");
            }
        }
        UHID_STOP => {
            eprintln!("hidd: UHID_STOP");
        }
        UHID_OPEN => {
            eprintln!("hidd: UHID_OPEN");
        }
        UHID_CLOSE => {
            eprintln!("hidd: UHID_CLOSE");
        }
        UHID_OUTPUT => {
            let size = read_u16(event, 4 + UHID_OUTPUT_DATA_MAX)
                .map(usize::from)
                .unwrap_or(0)
                .min(UHID_OUTPUT_DATA_MAX);
            let rtype = event
                .get(4 + UHID_OUTPUT_DATA_MAX + 2)
                .copied()
                .unwrap_or(0);
            let data = &event[4..4 + size];

            *dropped_output_reports += 1;
            if *dropped_output_reports <= 3 || *dropped_output_reports % 100 == 0 {
                if let Some(parsed) = OutputReport::parse(data) {
                    eprintln!(
                        "hidd: dropped UHID_OUTPUT rtype={rtype} rumble={{lt:{}, rt:{}, weak:{}, strong:{}}} count={}",
                        parsed.left_trigger_magnitude,
                        parsed.right_trigger_magnitude,
                        parsed.weak_motor_magnitude,
                        parsed.strong_motor_magnitude,
                        *dropped_output_reports
                    );
                } else {
                    let report_id = data.first().copied().unwrap_or(0);
                    eprintln!(
                        "hidd: dropped UHID_OUTPUT rtype={rtype} report_id=0x{report_id:02x} size={size} count={}",
                        *dropped_output_reports
                    );
                }
            }
        }
        UHID_GET_REPORT => {
            let id = read_u32(event, 4).unwrap_or(0);
            let rnum = event.get(8).copied().unwrap_or(0);
            let rtype = event.get(9).copied().unwrap_or(0);
            eprintln!(
                "hidd: UHID_GET_REPORT id={id} rnum={rnum} rtype={rtype}; replying not supported"
            );
            write_get_report_reply(io, id, UHID_ERR_NOT_SUPPORTED, &[])?;
        }
        UHID_SET_REPORT => {
            let id = read_u32(event, 4).unwrap_or(0);
            let rnum = event.get(8).copied().unwrap_or(0);
            let rtype = event.get(9).copied().unwrap_or(0);
            let size = read_u16(event, 10).map(usize::from).unwrap_or(0);
            eprintln!(
                "hidd: UHID_SET_REPORT id={id} rnum={rnum} rtype={rtype} size={size}; dropping payload"
            );
            // Accept writes but ignore payload in MVP (no haptics implementation).
            write_set_report_reply(io, id, 0)?;
        }
        _ => {}
    }

    Ok(())
}

fn write_get_report_reply(io: &mut std::fs::File, id: u32, err: u16, data: &[u8]) -> Result<()> {
    if data.len() > UHID_OUTPUT_DATA_MAX {
        return Err(anyhow!("get-report reply too large: {}", data.len()));
    }

    let mut event = Vec::with_capacity(12 + data.len());
    event.extend_from_slice(&UHID_GET_REPORT_REPLY.to_ne_bytes());
    event.extend_from_slice(&id.to_ne_bytes());
    event.extend_from_slice(&err.to_ne_bytes());
    event.extend_from_slice(&(data.len() as u16).to_ne_bytes());
    event.extend_from_slice(data);

    io.write_all(&event)
        .map_err(|e| anyhow!("failed to write UHID_GET_REPORT_REPLY: {e}"))
}

fn write_set_report_reply(io: &mut std::fs::File, id: u32, err: u16) -> Result<()> {
    let mut event = Vec::with_capacity(10);
    event.extend_from_slice(&UHID_SET_REPORT_REPLY.to_ne_bytes());
    event.extend_from_slice(&id.to_ne_bytes());
    event.extend_from_slice(&err.to_ne_bytes());

    io.write_all(&event)
        .map_err(|e| anyhow!("failed to write UHID_SET_REPORT_REPLY: {e}"))
}

fn read_u16(buf: &[u8], offset: usize) -> Option<u16> {
    let bytes = buf.get(offset..offset + 2)?;
    Some(u16::from_ne_bytes([bytes[0], bytes[1]]))
}

fn read_u32(buf: &[u8], offset: usize) -> Option<u32> {
    let bytes = buf.get(offset..offset + 4)?;
    Some(u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(buf: &[u8], offset: usize) -> Option<u64> {
    let bytes = buf.get(offset..offset + 8)?;
    Some(u64::from_ne_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

#[derive(Debug, Clone)]
struct PatternState {
    mode: PatternStateMode,
    base: InputReport,
}

#[derive(Debug, Clone)]
enum PatternStateMode {
    ButtonToggle {
        button_index: u8,
        period_reports: u16,
        report_counter: u64,
    },
    AxisSweep {
        axis: AxisName,
        step: i16,
        min: i16,
        max: i16,
        value: i16,
        direction: i16,
    },
}

impl PatternState {
    fn new(pattern: &PatternConfig) -> Self {
        let mode = match *pattern {
            PatternConfig::ButtonToggle {
                button_index,
                period_reports,
            } => PatternStateMode::ButtonToggle {
                button_index,
                period_reports,
                report_counter: 0,
            },
            PatternConfig::AxisSweep {
                axis,
                step,
                min,
                max,
            } => PatternStateMode::AxisSweep {
                axis,
                step,
                min,
                max,
                value: min,
                direction: 1,
            },
        };
        Self {
            mode,
            base: InputReport::default(),
        }
    }

    fn next_report(&mut self) -> InputReport {
        let mut report = self.base;
        match &mut self.mode {
            PatternStateMode::ButtonToggle {
                button_index,
                period_reports,
                report_counter,
            } => {
                let phase = (*report_counter / u64::from(*period_reports)) % 2;
                if phase == 1 {
                    report.buttons |= 1u16 << u16::from(*button_index);
                }
                *report_counter += 1;
            }
            PatternStateMode::AxisSweep {
                axis,
                step,
                min,
                max,
                value,
                direction,
            } => {
                apply_axis_value(&mut report, *axis, *value);

                let mut next = *value + (*step * *direction);
                if next >= *max {
                    next = *max;
                    *direction = -1;
                } else if next <= *min {
                    next = *min;
                    *direction = 1;
                }
                *value = next;
            }
        }
        report
    }
}

fn apply_axis_value(report: &mut InputReport, axis: AxisName, value: i16) {
    match axis {
        AxisName::Lx => report.lx = value,
        AxisName::Ly => report.ly = value,
        AxisName::Rx => report.rx = value,
        AxisName::Ry => report.ry = value,
        AxisName::Lt => report.lt = value.max(0) as u16,
        AxisName::Rt => report.rt = value.max(0) as u16,
    }
}

#[cfg(test)]
mod tests {
    use super::PatternState;
    use common::config::{AxisName, PatternConfig};

    #[test]
    fn button_toggle_changes_state_over_time() {
        let mut pattern = PatternState::new(&PatternConfig::ButtonToggle {
            button_index: 2,
            period_reports: 3,
        });

        let mut reports = Vec::new();
        for _ in 0..7 {
            reports.push(pattern.next_report().buttons);
        }

        assert_eq!(reports[0], 0);
        assert_eq!(reports[1], 0);
        assert_eq!(reports[2], 0);
        assert_eq!(reports[3], 1 << 2);
        assert_eq!(reports[4], 1 << 2);
        assert_eq!(reports[5], 1 << 2);
        assert_eq!(reports[6], 0);
    }

    #[test]
    fn axis_sweep_bounces_between_limits() {
        let mut pattern = PatternState::new(&PatternConfig::AxisSweep {
            axis: AxisName::Lx,
            step: 3,
            min: -3,
            max: 3,
        });

        let mut values = Vec::new();
        for _ in 0..6 {
            values.push(pattern.next_report().lx);
        }

        assert_eq!(values, vec![-3, 0, 3, 0, -3, 0]);
    }
}
