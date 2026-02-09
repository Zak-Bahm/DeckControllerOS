#![forbid(unsafe_code)]

use std::env;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use common::config::{HidConfig, DEFAULT_HID_CONFIG_PATH};

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
    let mut config_path = DEFAULT_HID_CONFIG_PATH.to_string();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--validate-config" => validate_config = true,
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

    if !validate_config {
        return Err(anyhow!(
            "runtime daemon not implemented yet; use --validate-config [--config <path>]"
        ));
    }

    let cfg = HidConfig::load_from_path(&config_path)?;
    println!(
        "HID config OK: name=\"{}\" vid=0x{:04x} pid=0x{:04x} rate={}Hz",
        cfg.device.name, cfg.device.vendor_id, cfg.device.product_id, cfg.report.rate_hz
    );
    Ok(())
}

fn print_help() {
    println!("Usage:");
    println!("  hidd --validate-config [--config <path>]");
    println!("Defaults:");
    println!("  --config {}", DEFAULT_HID_CONFIG_PATH);
}
