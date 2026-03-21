#[cfg(feature = "deck")]
use slint::Model;

slint::include_modules!();

#[cfg(feature = "deck")]
mod bluez;

fn main() -> anyhow::Result<()> {
    // Handle --list-devices CLI before any GUI setup
    if std::env::args().any(|a| a == "--list-devices") {
        #[cfg(feature = "deck")]
        return list_devices_cli();

        #[cfg(not(feature = "deck"))]
        anyhow::bail!("--list-devices requires the 'deck' feature (BlueZ support)");
    }

    tracing_subscriber::fmt::init();
    tracing::info!("Starting controlleros-gui");

    let window = MainWindow::new()?;

    #[cfg(feature = "deck")]
    let rt_handle = {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        let handle = rt.handle().clone();
        // Keep runtime alive in a background thread
        std::thread::spawn(move || rt.block_on(std::future::pending::<()>()));
        handle
    };

    // Initial device load
    #[cfg(feature = "deck")]
    {
        let w = window.as_weak();
        let h = rt_handle.clone();
        rt_handle.spawn(async move {
            match refresh_devices(&w).await {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!("Initial device load failed: {e}");
                    set_status(&w, &format!("Error: {e}"));
                }
            }
            // Start polling loop
            poll_devices(w, h).await;
        });
    }

    // Disconnect callback
    #[cfg(feature = "deck")]
    window.on_disconnect({
        let w = window.as_weak();
        let h = rt_handle.clone();
        move |index| {
            tracing::info!("Disconnect requested for device index {index}");
            let w = w.clone();
            let h = h.clone();
            h.spawn(async move {
                set_busy(&w, true);
                set_status(&w, "Disconnecting...");
                let obj_path = get_device_field(&w, index, |d| d.obj_path.to_string());
                if let Some(obj_path) = obj_path {
                    match async {
                        let conn = zbus::Connection::system().await?;
                        bluez::disconnect_device(&conn, &obj_path).await
                    }
                    .await
                    {
                        Ok(()) => {
                            set_status(&w, "Disconnected.");
                            let _ = refresh_devices(&w).await;
                        }
                        Err(e) => {
                            tracing::error!("Disconnect failed: {e}");
                            set_status(&w, &format!("Error: {e}"));
                        }
                    }
                }
                set_busy(&w, false);
            });
        }
    });

    #[cfg(not(feature = "deck"))]
    window.on_disconnect({
        let w = window.as_weak();
        move |index| {
            tracing::info!("Disconnect requested for device index {index}");
            if let Some(w) = w.upgrade() {
                w.set_status_text(format!("Disconnecting device {index}...").into());
            }
        }
    });

    // Confirm action callback (forget, restart-stack, restart, power-off)
    #[cfg(feature = "deck")]
    window.on_confirm_action({
        let w = window.as_weak();
        let h = rt_handle.clone();
        move || {
            let Some(win) = w.upgrade() else { return };
            let action = win.get_pending_action();
            let index = win.get_pending_index();
            tracing::info!("Confirmed action: {action}, index: {index}");

            win.set_confirm_visible(false);
            win.set_pending_action("".into());
            win.set_pending_index(-1);

            match action.as_str() {
                "forget" => {
                    let w = w.clone();
                    let h = h.clone();
                    h.spawn(async move {
                        set_busy(&w, true);
                        set_status(&w, "Forgetting device...");
                        let paths = get_device_field(&w, index, |d| d.obj_path.to_string());
                        if let Some(device_path) = paths {
                            // Derive adapter path from device path
                            // e.g. /org/bluez/hci0/dev_XX -> /org/bluez/hci0
                            let adapter_path = device_path
                                .rfind('/')
                                .map(|i| &device_path[..i])
                                .unwrap_or("/org/bluez/hci0");
                            match async {
                                let conn = zbus::Connection::system().await?;
                                bluez::remove_device(&conn, adapter_path, &device_path).await
                            }
                            .await
                            {
                                Ok(()) => {
                                    set_status(&w, "Device forgotten.");
                                    let _ = refresh_devices(&w).await;
                                }
                                Err(e) => {
                                    tracing::error!("Forget failed: {e}");
                                    set_status(&w, &format!("Error: {e}"));
                                }
                            }
                        }
                        set_busy(&w, false);
                    });
                }
                "restart-stack" => {
                    tracing::info!("Restart stack requested");
                    win.set_status_text("Restarting Bluetooth stack...".into());
                }
                "restart" => {
                    tracing::info!("System restart requested");
                    win.set_status_text("Restarting...".into());
                }
                "power-off" => {
                    tracing::info!("System power off requested");
                    win.set_status_text("Powering off...".into());
                }
                _ => {}
            }
        }
    });

    #[cfg(not(feature = "deck"))]
    window.on_confirm_action({
        let w = window.as_weak();
        move || {
            let Some(w) = w.upgrade() else { return };
            let action = w.get_pending_action();
            let index = w.get_pending_index();
            tracing::info!("Confirmed action: {action}, index: {index}");

            match action.as_str() {
                "forget" => {
                    w.set_status_text(format!("Forgetting device {index}...").into());
                }
                "restart-stack" => {
                    w.set_status_text("Restarting Bluetooth stack...".into());
                }
                "restart" => {
                    w.set_status_text("Restarting...".into());
                }
                "power-off" => {
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

#[cfg(feature = "deck")]
fn list_devices_cli() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let connection = zbus::Connection::system().await?;
        let devices = bluez::list_paired_devices(&connection).await?;

        if devices.is_empty() {
            println!("No paired devices found.");
        } else {
            for d in &devices {
                let status = if d.connected {
                    "Connected"
                } else {
                    "Disconnected"
                };
                println!("{}\t{}\t{}\t{}", d.name, d.address, status, d.obj_path);
            }
        }

        Ok(())
    })
}

// --- Helpers for updating UI from async tasks (deck feature) ---

#[cfg(feature = "deck")]
fn set_status(w: &slint::Weak<MainWindow>, msg: &str) {
    let msg = msg.to_string();
    let w = w.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(w) = w.upgrade() {
            w.set_status_text(msg.into());
        }
    })
    .ok();
}

#[cfg(feature = "deck")]
fn set_busy(w: &slint::Weak<MainWindow>, busy: bool) {
    let w = w.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(w) = w.upgrade() {
            w.set_busy(busy);
        }
    })
    .ok();
}

#[cfg(feature = "deck")]
fn get_device_field<F, R>(w: &slint::Weak<MainWindow>, index: i32, f: F) -> Option<R>
where
    F: FnOnce(&BtDeviceModel) -> R + Send + 'static,
    R: Send + 'static,
{
    let w = w.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    slint::invoke_from_event_loop(move || {
        let result = w.upgrade().and_then(|w| {
            let devices = w.get_devices();
            let device = devices.row_data(index as usize)?;
            Some(f(&device))
        });
        let _ = tx.send(result);
    })
    .ok()?;
    rx.recv().ok()?
}

#[cfg(feature = "deck")]
async fn refresh_devices(w: &slint::Weak<MainWindow>) -> anyhow::Result<()> {
    let conn = zbus::Connection::system().await?;
    let devices = bluez::list_paired_devices(&conn).await?;
    let adapter = bluez::adapter_info(&conn).await.ok();

    let any_connected = devices.iter().any(|d| d.connected);

    // Determine status text based on adapter pairable state
    let status = if !any_connected {
        if let Some(ref info) = adapter {
            if info.pairable {
                format!("Pairable as {}", info.alias)
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let items: Vec<BtDeviceModel> = devices
        .iter()
        .map(|d| BtDeviceModel {
            name: d.name.clone().into(),
            address: d.address.clone().into(),
            connected: d.connected,
            obj_path: d.obj_path.clone().into(),
        })
        .collect();
    let w = w.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(w) = w.upgrade() {
            let model = slint::ModelRc::new(slint::VecModel::from(items));
            w.set_devices(model);
            // Only update status if not busy (don't overwrite in-progress operation messages)
            if !w.get_busy() {
                w.set_status_text(status.into());
            }
        }
    })
    .ok();
    Ok(())
}

#[cfg(feature = "deck")]
async fn poll_devices(w: slint::Weak<MainWindow>, _h: tokio::runtime::Handle) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
    loop {
        interval.tick().await;
        match refresh_devices(&w).await {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!("Polling refresh failed: {e}");
            }
        }
    }
}
