#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use common::config::{HidConfig, DEFAULT_HID_CONFIG_PATH};
use common::hid::{
    XBOX_BUTTON_A, XBOX_BUTTON_B, XBOX_BUTTON_HOME, XBOX_BUTTON_LB, XBOX_BUTTON_LS, XBOX_BUTTON_RB,
    XBOX_BUTTON_RS, XBOX_BUTTON_SELECT, XBOX_BUTTON_START, XBOX_BUTTON_X, XBOX_BUTTON_Y,
};
use input::{discover_devices, InputReader, MappingConfig};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("controllerosctl: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse(env::args().skip(1))?;
    match args.cmd {
        CommandKind::HidSelfTest => run_hid_self_test(&args),
        CommandKind::InputList => run_input_list(),
        CommandKind::InputMonitor => run_input_monitor(&args),
        CommandKind::Help => {
            print_help();
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
struct Args {
    cmd: CommandKind,
    config_path: String,
    mapping_config_path: String,
    hidd_path: PathBuf,
    pattern_seconds: u64,
}

const DEFAULT_MAPPING_CONFIG_PATH: &str = "/etc/controlleros/mapping/xbox.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandKind {
    HidSelfTest,
    InputList,
    InputMonitor,
    Help,
}

impl Args {
    fn parse<I>(mut args: I) -> Result<Self>
    where
        I: Iterator<Item = String>,
    {
        let mut config_path = DEFAULT_HID_CONFIG_PATH.to_string();
        let mut mapping_config_path = DEFAULT_MAPPING_CONFIG_PATH.to_string();
        let mut hidd_path = infer_hidd_path();
        let mut pattern_seconds = 2u64;

        let first = args.next();
        let mut cmd = match first.as_deref() {
            None | Some("--help") | Some("-h") => CommandKind::Help,
            Some("hid") => match args.next().as_deref() {
                Some("self-test") => CommandKind::HidSelfTest,
                Some(other) => return Err(anyhow!("unknown hid subcommand: {other}")),
                None => return Err(anyhow!("missing hid subcommand (expected: self-test)")),
            },
            Some("input") => match args.next().as_deref() {
                Some("list") => CommandKind::InputList,
                Some("monitor") => CommandKind::InputMonitor,
                Some(other) => return Err(anyhow!("unknown input subcommand: {other}")),
                None => {
                    return Err(anyhow!(
                        "missing input subcommand (expected: list, monitor)"
                    ))
                }
            },
            Some(other) => return Err(anyhow!("unknown command: {other}")),
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => {
                    config_path = args
                        .next()
                        .ok_or_else(|| anyhow!("missing value for --config"))?;
                }
                "--mapping-config" => {
                    mapping_config_path = args
                        .next()
                        .ok_or_else(|| anyhow!("missing value for --mapping-config"))?;
                }
                "--hidd" => {
                    hidd_path = PathBuf::from(
                        args.next()
                            .ok_or_else(|| anyhow!("missing value for --hidd"))?,
                    );
                }
                "--pattern-seconds" => {
                    let raw = args
                        .next()
                        .ok_or_else(|| anyhow!("missing value for --pattern-seconds"))?;
                    pattern_seconds = raw
                        .parse::<u64>()
                        .map_err(|_| anyhow!("invalid --pattern-seconds value: {raw}"))?;
                    if pattern_seconds == 0 || pattern_seconds > 30 {
                        return Err(anyhow!("--pattern-seconds must be in 1..=30"));
                    }
                }
                "--help" | "-h" => {
                    cmd = CommandKind::Help;
                }
                other => return Err(anyhow!("unknown argument: {other}")),
            }
        }

        Ok(Self {
            cmd,
            config_path,
            mapping_config_path,
            hidd_path,
            pattern_seconds,
        })
    }
}

fn infer_hidd_path() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let sibling = parent.join("hidd");
            if sibling.exists() {
                return sibling;
            }
        }
    }
    PathBuf::from("hidd")
}

fn run_hid_self_test(args: &Args) -> Result<()> {
    let cfg = HidConfig::load_from_path(&args.config_path)?;
    println!("profile_mode={}", cfg.profile.mode.as_str());
    println!(
        "profile_identity=vid=0x{:04x} pid=0x{:04x} version=0x{:04x} country={}",
        cfg.profile.vendor_id, cfg.profile.product_id, cfg.profile.version, cfg.profile.country
    );
    println!(
        "descriptor_len={}",
        cfg.profile.mode.report_descriptor().len()
    );
    println!("report_len={}", cfg.profile.mode.input_report_len());

    run_hidd(args, &["--self-test", "--config", &args.config_path])?;
    println!("UHID OK");

    let mut child = Command::new(&args.hidd_path)
        .args(["--config", args.config_path.as_str()])
        .spawn()
        .map_err(|e| anyhow!("failed to spawn {:?}: {e}", args.hidd_path))?;

    thread::sleep(Duration::from_secs(args.pattern_seconds));

    if let Some(status) = child
        .try_wait()
        .map_err(|e| anyhow!("failed waiting on hidd process: {e}"))?
    {
        if !status.success() {
            return Err(anyhow!(
                "hidd exited before pattern window with status {status}"
            ));
        }
        return Err(anyhow!("hidd exited early before pattern window completed"));
    }

    child
        .kill()
        .map_err(|e| anyhow!("failed to stop hidd after pattern window: {e}"))?;

    let status = child
        .wait()
        .map_err(|e| anyhow!("failed waiting for hidd shutdown: {e}"))?;
    if !status.success() {
        // Non-zero here is expected because we terminate it after the short run.
        println!("pattern_run=OK duration={}s", args.pattern_seconds);
        return Ok(());
    }

    println!("pattern_run=OK duration={}s", args.pattern_seconds);
    Ok(())
}

fn run_hidd(args: &Args, hidd_args: &[&str]) -> Result<()> {
    let status = Command::new(&args.hidd_path)
        .args(hidd_args)
        .status()
        .map_err(|e| anyhow!("failed to execute {:?}: {e}", args.hidd_path))?;

    if !status.success() {
        return Err(anyhow!(
            "hidd command failed: {:?} {:?} => {status}",
            args.hidd_path,
            hidd_args
        ));
    }

    Ok(())
}

fn run_input_list() -> Result<()> {
    let devices = discover_devices();
    if devices.is_empty() {
        println!("no input devices found");
        return Ok(());
    }
    for dev in &devices {
        let marker = if dev.is_deck_gamepad { " *" } else { "" };
        println!(
            "{}: \"{}\" vid=0x{:04x} pid=0x{:04x} {}{}",
            dev.path.display(),
            dev.name,
            dev.vendor,
            dev.product,
            dev.caps_summary,
            marker,
        );
    }
    println!();
    println!("* = detected as Steam Deck gamepad");
    Ok(())
}

fn run_input_monitor(args: &Args) -> Result<()> {
    let config = MappingConfig::from_file(&args.mapping_config_path)
        .map_err(|e| anyhow!("mapping config: {e}"))?;
    let reader = InputReader::new(config).map_err(|e| anyhow!("{e}"))?;

    println!("monitoring input (Ctrl+C to stop)...");

    // Give the event loop thread a moment to start.
    thread::sleep(Duration::from_millis(50));

    // Print initial state so we know the pipeline works.
    let mut prev = reader.current_report();
    println!("initial state:");
    print_report(&prev);
    println!();

    loop {
        let report = reader.current_report();

        // Only print when something changed.
        if report_differs(&prev, &report) {
            print_report(&report);
            prev = report;
        }

        thread::sleep(Duration::from_millis(16)); // ~60 Hz poll
    }
}

fn report_differs(a: &common::hid::InputReport, b: &common::hid::InputReport) -> bool {
    a.buttons != b.buttons
        || a.hat != b.hat
        || a.lx != b.lx
        || a.ly != b.ly
        || a.rx != b.rx
        || a.ry != b.ry
        || a.lt != b.lt
        || a.rt != b.rt
}

fn print_report(r: &common::hid::InputReport) {
    let buttons = format_buttons(r.buttons);
    let hat = format_hat(r.hat);
    println!(
        "LX:{:+6} LY:{:+6} RX:{:+6} RY:{:+6} LT:{:4} RT:{:4} hat:{} {}",
        r.lx, r.ly, r.rx, r.ry, r.lt, r.rt, hat, buttons,
    );
}

fn format_buttons(bits: u16) -> String {
    let names: &[(u16, &str)] = &[
        (XBOX_BUTTON_A, "A"),
        (XBOX_BUTTON_B, "B"),
        (XBOX_BUTTON_X, "X"),
        (XBOX_BUTTON_Y, "Y"),
        (XBOX_BUTTON_LB, "LB"),
        (XBOX_BUTTON_RB, "RB"),
        (XBOX_BUTTON_SELECT, "Back"),
        (XBOX_BUTTON_START, "Start"),
        (XBOX_BUTTON_LS, "LS"),
        (XBOX_BUTTON_RS, "RS"),
        (XBOX_BUTTON_HOME, "Home"),
    ];
    let pressed: Vec<&str> = names
        .iter()
        .filter(|(mask, _)| bits & mask != 0)
        .map(|(_, name)| *name)
        .collect();
    if pressed.is_empty() {
        String::new()
    } else {
        format!("[{}]", pressed.join(" "))
    }
}

fn format_hat(hat: u8) -> &'static str {
    match hat {
        1 => "N ",
        2 => "NE",
        3 => "E ",
        4 => "SE",
        5 => "S ",
        6 => "SW",
        7 => "W ",
        8 => "NW",
        _ => "- ",
    }
}

fn print_help() {
    println!("Usage:");
    println!("  controllerosctl hid self-test [--config <path>] [--hidd <path>] [--pattern-seconds <1..30>]");
    println!("  controllerosctl input list");
    println!("  controllerosctl input monitor [--mapping-config <path>]");
    println!("Defaults:");
    println!("  --config {}", DEFAULT_HID_CONFIG_PATH);
    println!("  --mapping-config {}", DEFAULT_MAPPING_CONFIG_PATH);
    println!("  --hidd sibling ./hidd (or PATH lookup)");
    println!("  --pattern-seconds 2");
}

#[cfg(test)]
mod tests {
    use super::{Args, CommandKind};

    #[test]
    fn parses_hid_self_test_defaults() {
        let args = Args::parse(vec!["hid".into(), "self-test".into()].into_iter())
            .expect("parse should succeed");

        assert_eq!(args.cmd, CommandKind::HidSelfTest);
        assert_eq!(args.pattern_seconds, 2);
    }

    #[test]
    fn rejects_invalid_pattern_seconds() {
        let err = Args::parse(
            vec![
                "hid".into(),
                "self-test".into(),
                "--pattern-seconds".into(),
                "0".into(),
            ]
            .into_iter(),
        )
        .expect_err("pattern-seconds=0 should fail");

        assert!(err.to_string().contains("1..=30"));
    }

    #[test]
    fn parses_input_list() {
        let args = Args::parse(vec!["input".into(), "list".into()].into_iter())
            .expect("parse should succeed");
        assert_eq!(args.cmd, CommandKind::InputList);
    }

    #[test]
    fn parses_input_monitor() {
        let args = Args::parse(vec!["input".into(), "monitor".into()].into_iter())
            .expect("parse should succeed");
        assert_eq!(args.cmd, CommandKind::InputMonitor);
    }

    #[test]
    fn parses_input_monitor_with_mapping_config() {
        let args = Args::parse(
            vec![
                "input".into(),
                "monitor".into(),
                "--mapping-config".into(),
                "/tmp/test.toml".into(),
            ]
            .into_iter(),
        )
        .expect("parse should succeed");
        assert_eq!(args.cmd, CommandKind::InputMonitor);
        assert_eq!(args.mapping_config_path, "/tmp/test.toml");
    }
}
