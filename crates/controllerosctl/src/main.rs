#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use common::config::DEFAULT_HID_CONFIG_PATH;
use common::hid::{HID_REPORT_DESCRIPTOR, INPUT_REPORT_LEN};

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
    hidd_path: PathBuf,
    pattern_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandKind {
    HidSelfTest,
    Help,
}

impl Args {
    fn parse<I>(mut args: I) -> Result<Self>
    where
        I: Iterator<Item = String>,
    {
        let mut config_path = DEFAULT_HID_CONFIG_PATH.to_string();
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
            Some(other) => return Err(anyhow!("unknown command: {other}")),
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => {
                    config_path = args
                        .next()
                        .ok_or_else(|| anyhow!("missing value for --config"))?;
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
    println!("descriptor_len={}", HID_REPORT_DESCRIPTOR.len());
    println!("report_len={INPUT_REPORT_LEN}");

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

fn print_help() {
    println!("Usage:");
    println!("  controllerosctl hid self-test [--config <path>] [--hidd <path>] [--pattern-seconds <1..30>]");
    println!("Defaults:");
    println!("  --config {}", DEFAULT_HID_CONFIG_PATH);
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
}
