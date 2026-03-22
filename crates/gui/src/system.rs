use anyhow::Result;
use tokio::process::Command;

pub async fn reload_stack() -> Result<()> {
    tracing::info!("Stopping hidd...");
    Command::new("/etc/init.d/S45hidd")
        .arg("stop")
        .status()
        .await?;

    tracing::info!("Stopping bluetoothd...");
    Command::new("/etc/init.d/S40bluetoothd")
        .arg("stop")
        .status()
        .await?;

    tracing::info!("Starting bluetoothd...");
    Command::new("/etc/init.d/S40bluetoothd")
        .arg("start")
        .status()
        .await?;

    tracing::info!("Starting hidd...");
    Command::new("/etc/init.d/S45hidd")
        .arg("start")
        .status()
        .await?;

    tracing::info!("Stack reload complete, re-execing GUI...");
    re_exec_self()
}

pub async fn system_poweroff() -> Result<()> {
    tracing::info!("Executing /sbin/poweroff...");
    Command::new("/sbin/poweroff").status().await?;
    Ok(())
}

fn re_exec_self() -> Result<()> {
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe()?;
    let err = std::process::Command::new(&exe).exec();
    anyhow::bail!("exec failed: {err}");
}
