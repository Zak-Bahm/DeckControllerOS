slint::include_modules!();

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting controlleros-gui");

    let window = MainWindow::new()?;

    window.on_disconnect({
        let w = window.as_weak();
        move |index| {
            tracing::info!("Disconnect requested for device index {index}");
            if let Some(w) = w.upgrade() {
                w.set_status_text(format!("Disconnecting device {index}...").into());
            }
        }
    });

    window.on_reconnect({
        let w = window.as_weak();
        move |index| {
            tracing::info!("Reconnect requested for device index {index}");
            if let Some(w) = w.upgrade() {
                w.set_status_text(format!("Reconnecting device {index}...").into());
            }
        }
    });

    window.on_confirm_action({
        let w = window.as_weak();
        move || {
            let Some(w) = w.upgrade() else { return };
            let action = w.get_pending_action();
            let index = w.get_pending_index();
            tracing::info!("Confirmed action: {action}, index: {index}");

            match action.as_str() {
                "forget" => {
                    tracing::info!("Forget requested for device index {index}");
                    w.set_status_text(format!("Forgetting device {index}...").into());
                }
                "restart-stack" => {
                    tracing::info!("Restart stack requested");
                    w.set_status_text("Restarting Bluetooth stack...".into());
                }
                "restart" => {
                    tracing::info!("System restart requested");
                    w.set_status_text("Restarting...".into());
                }
                "power-off" => {
                    tracing::info!("System power off requested");
                    w.set_status_text("Powering off...".into());
                }
                _ => {}
            }

            w.set_confirm_visible(false);
            w.set_pending_action("".into());
            w.set_pending_index(-1);
        }
    });

    window.run()?;

    Ok(())
}
