#![forbid(unsafe_code)]

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use common::config::{AxisName, HidConfig, PatternConfig, DEFAULT_HID_CONFIG_PATH};
use common::hid::{InputReport, HID_REPORT_DESCRIPTOR};

const DEV_UHID: &str = "/dev/uhid";
const UHID_DESTROY: u32 = 1;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;
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
            "HID config OK: name=\"{}\" vid=0x{:04x} pid=0x{:04x} rate={}Hz",
            cfg.device.name, cfg.device.vendor_id, cfg.device.product_id, cfg.report.rate_hz
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
    println!(
        "hidd started: name=\"{}\" vid=0x{:04x} pid=0x{:04x} rate={}Hz",
        cfg.device.name, cfg.device.vendor_id, cfg.device.product_id, cfg.report.rate_hz
    );

    let period = Duration::from_nanos(1_000_000_000u64 / u64::from(cfg.report.rate_hz));
    let mut pattern = PatternState::new(&cfg.pattern);
    let mut next_tick = Instant::now();

    loop {
        let report = pattern.next_report();
        uhid.send_input_report(&report.to_bytes())?;
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

    fn send_input_report(&mut self, report: &[u8]) -> Result<()> {
        if report.len() > 4096 {
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

    let mut payload = vec![0u8; CREATE2_PAYLOAD_LEN];
    write_padded(&mut payload[OFF_NAME..OFF_PHYS], &cfg.device.name);
    write_padded(&mut payload[OFF_PHYS..OFF_UNIQ], "bluetooth");
    payload[OFF_RD_SIZE..OFF_RD_SIZE + 2]
        .copy_from_slice(&(HID_REPORT_DESCRIPTOR.len() as u16).to_ne_bytes());
    payload[OFF_BUS..OFF_BUS + 2].copy_from_slice(&BUS_BLUETOOTH.to_ne_bytes());
    payload[OFF_VENDOR..OFF_VENDOR + 4]
        .copy_from_slice(&(u32::from(cfg.device.vendor_id)).to_ne_bytes());
    payload[OFF_PRODUCT..OFF_PRODUCT + 4]
        .copy_from_slice(&(u32::from(cfg.device.product_id)).to_ne_bytes());
    payload[OFF_VERSION..OFF_VERSION + 4].copy_from_slice(&1u32.to_ne_bytes());
    payload[OFF_COUNTRY..OFF_COUNTRY + 4].copy_from_slice(&0u32.to_ne_bytes());
    payload[OFF_RD_DATA..OFF_RD_DATA + HID_REPORT_DESCRIPTOR.len()]
        .copy_from_slice(&HID_REPORT_DESCRIPTOR);

    let mut event = Vec::with_capacity(4 + payload.len());
    event.extend_from_slice(&UHID_CREATE2.to_ne_bytes());
    event.extend_from_slice(&payload);
    event
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
        AxisName::Lx => report.lx = value as i8,
        AxisName::Ly => report.ly = value as i8,
        AxisName::Rx => report.rx = value as i8,
        AxisName::Ry => report.ry = value as i8,
        AxisName::Lt => report.lt = value as u8,
        AxisName::Rt => report.rt = value as u8,
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
