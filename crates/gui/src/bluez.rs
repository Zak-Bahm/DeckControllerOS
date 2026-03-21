use std::collections::HashMap;

use anyhow::{Context, Result};
use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone)]
pub struct BtDevice {
    pub name: String,
    pub address: String,
    pub connected: bool,
    pub obj_path: String,
}

const BLUEZ_SERVICE: &str = "org.bluez";
const BLUEZ_ROOT_PATH: &str = "/";
const BLUEZ_DEVICE_IFACE: &str = "org.bluez.Device1";
const BLUEZ_ADAPTER_IFACE: &str = "org.bluez.Adapter1";

/// List all paired Bluetooth devices via BlueZ's ObjectManager.
pub async fn list_paired_devices(connection: &zbus::Connection) -> Result<Vec<BtDevice>> {
    let proxy = zbus::fdo::ObjectManagerProxy::builder(connection)
        .destination(BLUEZ_SERVICE)?
        .path(BLUEZ_ROOT_PATH)?
        .build()
        .await
        .context("Failed to create ObjectManager proxy")?;

    let objects = proxy
        .get_managed_objects()
        .await
        .context("Failed to call GetManagedObjects")?;

    tracing::debug!("GetManagedObjects returned {} entries", objects.len());

    let mut devices = Vec::new();

    for (path, ifaces) in &objects {
        // Find entries that have the Device1 interface
        let device_props = ifaces
            .iter()
            .find(|(k, _)| k.as_str() == BLUEZ_DEVICE_IFACE);

        let Some((_, props)) = device_props else {
            continue;
        };

        let name = prop_string(props, "Name").unwrap_or_else(|| "Unknown".to_string());
        let address = prop_string(props, "Address").unwrap_or_default();
        let paired = prop_bool(props, "Paired").unwrap_or(false);
        let connected = prop_bool(props, "Connected").unwrap_or(false);

        tracing::debug!(
            "BlueZ device: {name} ({address}) paired={paired} connected={connected} path={path}"
        );

        // Only include paired devices
        if !paired {
            continue;
        }

        devices.push(BtDevice {
            name,
            address,
            connected,
            obj_path: path.to_string(),
        });
    }

    tracing::info!("Found {} paired device(s)", devices.len());
    Ok(devices)
}

#[derive(Debug, Clone)]
pub struct AdapterInfo {
    pub alias: String,
    pub pairable: bool,
}

/// Get the first adapter's alias and pairable status.
pub async fn adapter_info(connection: &zbus::Connection) -> Result<AdapterInfo> {
    let proxy = zbus::fdo::ObjectManagerProxy::builder(connection)
        .destination(BLUEZ_SERVICE)?
        .path(BLUEZ_ROOT_PATH)?
        .build()
        .await
        .context("Failed to create ObjectManager proxy")?;

    let objects = proxy
        .get_managed_objects()
        .await
        .context("Failed to call GetManagedObjects")?;

    for ifaces in objects.values() {
        let adapter_props = ifaces
            .iter()
            .find(|(k, _)| k.as_str() == BLUEZ_ADAPTER_IFACE);

        let Some((_, props)) = adapter_props else {
            continue;
        };

        let alias = prop_string(props, "Alias")
            .or_else(|| prop_string(props, "Name"))
            .unwrap_or_else(|| "ControllerOS".to_string());
        let pairable = prop_bool(props, "Pairable").unwrap_or(false);

        return Ok(AdapterInfo { alias, pairable });
    }

    anyhow::bail!("No BlueZ adapter found")
}

/// Disconnect a paired device.
pub async fn disconnect_device(connection: &zbus::Connection, obj_path: &str) -> Result<()> {
    let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(connection)
        .destination(BLUEZ_SERVICE)?
        .path(obj_path)?
        .interface(BLUEZ_DEVICE_IFACE)?
        .build()
        .await
        .context("Failed to create Device1 proxy")?;

    proxy
        .call_method("Disconnect", &())
        .await
        .context("Failed to call Disconnect")?;

    Ok(())
}

/// Remove (forget) a device from an adapter.
pub async fn remove_device(
    connection: &zbus::Connection,
    adapter_path: &str,
    device_path: &str,
) -> Result<()> {
    let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(connection)
        .destination(BLUEZ_SERVICE)?
        .path(adapter_path)?
        .interface(BLUEZ_ADAPTER_IFACE)?
        .build()
        .await
        .context("Failed to create Adapter1 proxy")?;

    let device_obj_path =
        zbus::zvariant::ObjectPath::try_from(device_path).context("Invalid device object path")?;

    proxy
        .call_method("RemoveDevice", &(device_obj_path))
        .await
        .context("Failed to call RemoveDevice")?;

    Ok(())
}

fn prop_string(props: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let val = props.get(key)?;
    if let Ok(s) = val.downcast_ref::<&str>() {
        return Some(s.to_string());
    }
    <String as TryFrom<OwnedValue>>::try_from(val.clone()).ok()
}

fn prop_bool(props: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    let val = props.get(key)?;
    if let Ok(b) = val.downcast_ref::<bool>() {
        return Some(b);
    }
    <bool as TryFrom<OwnedValue>>::try_from(val.clone()).ok()
}
