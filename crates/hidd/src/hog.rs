use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use common::config::HidConfig;
use common::hid::{
    OutputReport, XBOX_EXTRA_INPUT_PAYLOAD_LEN, XBOX_EXTRA_INPUT_REPORT_ID, XBOX_INPUT_PAYLOAD_LEN,
    XBOX_INPUT_REPORT_ID, XBOX_OUTPUT_PAYLOAD_LEN, XBOX_OUTPUT_REPORT_ID, XBOX_OUTPUT_REPORT_LEN,
    XBOX_STATUS_INPUT_PAYLOAD_LEN, XBOX_STATUS_INPUT_REPORT_ID,
};
use dbus::arg::{Append, AppendAll, Arg, IterAppend, PropMap, RefArg, Variant};
use dbus::blocking::stdintf::org_freedesktop_dbus::{
    ObjectManager as OrgFreedesktopObjectManager, PropertiesPropertiesChanged,
};
use dbus::blocking::SyncConnection;
use dbus::channel::{MatchingReceiver, Sender};
use dbus::message::{MatchRule, SignalArgs};
use dbus::strings::{BusName, Interface, Member};
use dbus::{Message, MessageType, Path};
use dbus_crossroads::{Crossroads, IfaceToken, MethodErr};

const BLUEZ_SERVICE: &str = "org.bluez";
const BLUEZ_ROOT_PATH: &str = "/";
const BLUEZ_ADAPTER_IFACE: &str = "org.bluez.Adapter1";
const BLUEZ_GATT_MANAGER_IFACE: &str = "org.bluez.GattManager1";
const BLUEZ_LE_ADV_MANAGER_IFACE: &str = "org.bluez.LEAdvertisingManager1";
const DBUS_PROPERTIES_IFACE: &str = "org.freedesktop.DBus.Properties";

const GATT_SERVICE_IFACE: &str = "org.bluez.GattService1";
const GATT_CHARACTERISTIC_IFACE: &str = "org.bluez.GattCharacteristic1";
const GATT_DESCRIPTOR_IFACE: &str = "org.bluez.GattDescriptor1";
const LE_ADVERTISEMENT_IFACE: &str = "org.bluez.LEAdvertisement1";

const BLUEZ_AGENT_MANAGER_IFACE: &str = "org.bluez.AgentManager1";
const BLUEZ_AGENT_IFACE: &str = "org.bluez.Agent1";

const APP_PATH: &str = "/org/controlleros/hid";
const ADVERTISEMENT_PATH: &str = "/org/controlleros/advertisement0";
const AGENT_PATH: &str = "/org/controlleros/agent";

const HID_SERVICE_PATH: &str = "/org/controlleros/hid/service0";
const HID_PROTOCOL_MODE_CHAR_PATH: &str = "/org/controlleros/hid/service0/char0";
const HID_REPORT_MAP_CHAR_PATH: &str = "/org/controlleros/hid/service0/char1";
const HID_INFO_CHAR_PATH: &str = "/org/controlleros/hid/service0/char2";
const HID_CONTROL_POINT_CHAR_PATH: &str = "/org/controlleros/hid/service0/char3";
const HID_INPUT_REPORT_CHAR_PATH: &str = "/org/controlleros/hid/service0/char4";
const HID_OUTPUT_REPORT_CHAR_PATH: &str = "/org/controlleros/hid/service0/char5";
const HID_EXTRA_INPUT_REPORT_CHAR_PATH: &str = "/org/controlleros/hid/service0/char6";
const HID_STATUS_INPUT_REPORT_CHAR_PATH: &str = "/org/controlleros/hid/service0/char7";
const HID_INPUT_REPORT_REF_DESC_PATH: &str = "/org/controlleros/hid/service0/char4/desc0";
const HID_OUTPUT_REPORT_REF_DESC_PATH: &str = "/org/controlleros/hid/service0/char5/desc0";
const HID_EXTRA_INPUT_REPORT_REF_DESC_PATH: &str = "/org/controlleros/hid/service0/char6/desc0";
const HID_STATUS_INPUT_REPORT_REF_DESC_PATH: &str = "/org/controlleros/hid/service0/char7/desc0";

const BATTERY_SERVICE_PATH: &str = "/org/controlleros/hid/service1";
const BATTERY_LEVEL_CHAR_PATH: &str = "/org/controlleros/hid/service1/char0";
const DEVICE_INFO_SERVICE_PATH: &str = "/org/controlleros/hid/service2";
const PNP_ID_CHAR_PATH: &str = "/org/controlleros/hid/service2/char0";

const HID_SERVICE_UUID: &str = "00001812-0000-1000-8000-00805f9b34fb";
const BATTERY_SERVICE_UUID: &str = "0000180f-0000-1000-8000-00805f9b34fb";
const DEVICE_INFO_SERVICE_UUID: &str = "0000180a-0000-1000-8000-00805f9b34fb";
const HID_PROTOCOL_MODE_UUID: &str = "00002a4e-0000-1000-8000-00805f9b34fb";
const HID_REPORT_MAP_UUID: &str = "00002a4b-0000-1000-8000-00805f9b34fb";
const HID_INFO_UUID: &str = "00002a4a-0000-1000-8000-00805f9b34fb";
const HID_CONTROL_POINT_UUID: &str = "00002a4c-0000-1000-8000-00805f9b34fb";
const HID_REPORT_UUID: &str = "00002a4d-0000-1000-8000-00805f9b34fb";
const BATTERY_LEVEL_UUID: &str = "00002a19-0000-1000-8000-00805f9b34fb";
const PNP_ID_UUID: &str = "00002a50-0000-1000-8000-00805f9b34fb";
const REPORT_REFERENCE_UUID: &str = "00002908-0000-1000-8000-00805f9b34fb";

const HID_GAMEPAD_APPEARANCE: u16 = 0x03c4;
const REPORT_TYPE_INPUT: u8 = 0x01;
const REPORT_TYPE_OUTPUT: u8 = 0x02;
const HID_INFO_BCD: [u8; 2] = [0x11, 0x01];
const HID_FLAG_REMOTE_WAKE: u8 = 0x01;
const PNP_ID_VENDOR_SOURCE_USB: u8 = 0x02;

type SharedState = Arc<Mutex<HogState>>;

#[derive(Debug)]
struct InputReportState {
    notifying: bool,
    value: Vec<u8>,
}

#[derive(Debug)]
struct HogState {
    protocol_mode: u8,
    control_point: u8,
    battery_notifying: bool,
    input_reports: HashMap<u8, InputReportState>,
    output_reports: HashMap<u8, Vec<u8>>,
    battery_level: u8,
}

impl HogState {
    fn new() -> Self {
        let mut input_reports = HashMap::new();
        input_reports.insert(
            XBOX_INPUT_REPORT_ID,
            InputReportState {
                notifying: false,
                value: vec![0; XBOX_INPUT_PAYLOAD_LEN],
            },
        );
        input_reports.insert(
            XBOX_EXTRA_INPUT_REPORT_ID,
            InputReportState {
                notifying: false,
                value: vec![0; XBOX_EXTRA_INPUT_PAYLOAD_LEN],
            },
        );
        input_reports.insert(
            XBOX_STATUS_INPUT_REPORT_ID,
            InputReportState {
                notifying: false,
                value: vec![0; XBOX_STATUS_INPUT_PAYLOAD_LEN],
            },
        );

        let mut output_reports = HashMap::new();
        output_reports.insert(XBOX_OUTPUT_REPORT_ID, vec![0; XBOX_OUTPUT_PAYLOAD_LEN]);

        Self {
            protocol_mode: 0x01, // Report protocol mode
            control_point: 0,
            battery_notifying: false,
            input_reports,
            output_reports,
            battery_level: 100,
        }
    }
}

#[derive(Debug, Clone)]
struct GattServiceData {
    uuid: String,
    primary: bool,
    includes: Vec<Path<'static>>,
}

#[derive(Debug, Clone)]
struct GattCharacteristicData {
    uuid: String,
    service: Path<'static>,
    flags: Vec<String>,
    descriptors: Vec<Path<'static>>,
    kind: CharacteristicKind,
    state: SharedState,
}

#[derive(Debug, Clone)]
enum CharacteristicKind {
    ProtocolMode,
    ReportMap(Vec<u8>),
    HidInfo { country_code: u8 },
    ControlPoint,
    InputReport { report_id: u8 },
    OutputReport { report_id: u8 },
    BatteryLevel,
    PnpId { value: [u8; 7] },
}

#[derive(Debug, Clone)]
struct GattDescriptorData {
    uuid: String,
    characteristic: Path<'static>,
    flags: Vec<String>,
    kind: DescriptorKind,
}

#[derive(Debug, Clone, Copy)]
enum DescriptorKind {
    ReportReference { report_id: u8, report_type: u8 },
}

#[derive(Debug, Clone)]
struct AdvertisementData {
    local_name: String,
    service_uuids: Vec<String>,
    appearance: u16,
}

pub struct HogRuntime {
    conn: SyncConnection,
    state: SharedState,
    input_report_char_paths: HashMap<u8, Path<'static>>,
    adapter_path: Path<'static>,
}

impl HogRuntime {
    pub fn register(cfg: &HidConfig) -> Result<Self> {
        let conn = SyncConnection::new_system().map_err(|e| anyhow!("system bus: {e}"))?;
        let adapter_path = find_adapter_path(&conn)?;
        configure_adapter_for_hog(&conn, &adapter_path, cfg)?;
        let app_path = dbus_path(APP_PATH)?;
        let advertisement_path = dbus_path(ADVERTISEMENT_PATH)?;
        let input_report_char_path = dbus_path(HID_INPUT_REPORT_CHAR_PATH)?;
        let extra_input_report_char_path = dbus_path(HID_EXTRA_INPUT_REPORT_CHAR_PATH)?;
        let status_input_report_char_path = dbus_path(HID_STATUS_INPUT_REPORT_CHAR_PATH)?;
        let mut input_report_char_paths = HashMap::new();
        input_report_char_paths.insert(XBOX_INPUT_REPORT_ID, input_report_char_path.clone());
        input_report_char_paths.insert(
            XBOX_EXTRA_INPUT_REPORT_ID,
            extra_input_report_char_path.clone(),
        );
        input_report_char_paths.insert(
            XBOX_STATUS_INPUT_REPORT_ID,
            status_input_report_char_path.clone(),
        );
        let state = Arc::new(Mutex::new(HogState::new()));
        let country_code = u8::try_from(cfg.profile.country)
            .map_err(|_| anyhow!("profile.country must be in 0..=255 for HID Information"))?;
        let pnp_id_value = encode_pnp_id(
            cfg.profile.vendor_id,
            cfg.profile.product_id,
            cfg.profile.version,
        );

        let agent_path = dbus_path(AGENT_PATH)?;
        let mut crossroads = Crossroads::new();
        let service_iface = register_service_iface(&mut crossroads);
        let characteristic_iface = register_characteristic_iface(&mut crossroads);
        let descriptor_iface = register_descriptor_iface(&mut crossroads);
        let advertisement_iface = register_advertisement_iface(&mut crossroads);
        let agent_iface = register_agent_iface(&mut crossroads);

        crossroads.insert(APP_PATH, &[crossroads.object_manager()], ());

        crossroads.insert(
            HID_SERVICE_PATH,
            &[service_iface],
            GattServiceData {
                uuid: HID_SERVICE_UUID.to_string(),
                primary: true,
                includes: Vec::new(),
            },
        );
        crossroads.insert(
            BATTERY_SERVICE_PATH,
            &[service_iface],
            GattServiceData {
                uuid: BATTERY_SERVICE_UUID.to_string(),
                primary: true,
                includes: Vec::new(),
            },
        );
        crossroads.insert(
            DEVICE_INFO_SERVICE_PATH,
            &[service_iface],
            GattServiceData {
                uuid: DEVICE_INFO_SERVICE_UUID.to_string(),
                primary: true,
                includes: Vec::new(),
            },
        );

        crossroads.insert(
            HID_PROTOCOL_MODE_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_PROTOCOL_MODE_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string(), "encrypt-write".to_string()],
                descriptors: Vec::new(),
                kind: CharacteristicKind::ProtocolMode,
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            HID_REPORT_MAP_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_REPORT_MAP_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string()],
                descriptors: Vec::new(),
                kind: CharacteristicKind::ReportMap(cfg.profile.mode.report_descriptor().to_vec()),
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            HID_INFO_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_INFO_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string()],
                descriptors: Vec::new(),
                kind: CharacteristicKind::HidInfo { country_code },
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            HID_CONTROL_POINT_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_CONTROL_POINT_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-write".to_string()],
                descriptors: Vec::new(),
                kind: CharacteristicKind::ControlPoint,
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            HID_INPUT_REPORT_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_REPORT_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string(), "encrypt-notify".to_string()],
                descriptors: vec![dbus_path(HID_INPUT_REPORT_REF_DESC_PATH)?],
                kind: CharacteristicKind::InputReport {
                    report_id: XBOX_INPUT_REPORT_ID,
                },
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            HID_OUTPUT_REPORT_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_REPORT_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string(), "encrypt-write".to_string()],
                descriptors: vec![dbus_path(HID_OUTPUT_REPORT_REF_DESC_PATH)?],
                kind: CharacteristicKind::OutputReport {
                    report_id: XBOX_OUTPUT_REPORT_ID,
                },
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            HID_EXTRA_INPUT_REPORT_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_REPORT_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string(), "encrypt-notify".to_string()],
                descriptors: vec![dbus_path(HID_EXTRA_INPUT_REPORT_REF_DESC_PATH)?],
                kind: CharacteristicKind::InputReport {
                    report_id: XBOX_EXTRA_INPUT_REPORT_ID,
                },
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            HID_STATUS_INPUT_REPORT_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: HID_REPORT_UUID.to_string(),
                service: dbus_path(HID_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string(), "encrypt-notify".to_string()],
                descriptors: vec![dbus_path(HID_STATUS_INPUT_REPORT_REF_DESC_PATH)?],
                kind: CharacteristicKind::InputReport {
                    report_id: XBOX_STATUS_INPUT_REPORT_ID,
                },
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            BATTERY_LEVEL_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: BATTERY_LEVEL_UUID.to_string(),
                service: dbus_path(BATTERY_SERVICE_PATH)?,
                flags: vec!["encrypt-read".to_string(), "encrypt-notify".to_string()],
                descriptors: Vec::new(),
                kind: CharacteristicKind::BatteryLevel,
                state: Arc::clone(&state),
            },
        );
        crossroads.insert(
            PNP_ID_CHAR_PATH,
            &[characteristic_iface],
            GattCharacteristicData {
                uuid: PNP_ID_UUID.to_string(),
                service: dbus_path(DEVICE_INFO_SERVICE_PATH)?,
                flags: vec!["read".to_string()],
                descriptors: Vec::new(),
                kind: CharacteristicKind::PnpId {
                    value: pnp_id_value,
                },
                state: Arc::clone(&state),
            },
        );

        crossroads.insert(
            HID_INPUT_REPORT_REF_DESC_PATH,
            &[descriptor_iface],
            GattDescriptorData {
                uuid: REPORT_REFERENCE_UUID.to_string(),
                characteristic: dbus_path(HID_INPUT_REPORT_CHAR_PATH)?,
                flags: vec!["encrypt-read".to_string()],
                kind: DescriptorKind::ReportReference {
                    report_id: XBOX_INPUT_REPORT_ID,
                    report_type: REPORT_TYPE_INPUT,
                },
            },
        );
        crossroads.insert(
            HID_OUTPUT_REPORT_REF_DESC_PATH,
            &[descriptor_iface],
            GattDescriptorData {
                uuid: REPORT_REFERENCE_UUID.to_string(),
                characteristic: dbus_path(HID_OUTPUT_REPORT_CHAR_PATH)?,
                flags: vec!["encrypt-read".to_string()],
                kind: DescriptorKind::ReportReference {
                    report_id: XBOX_OUTPUT_REPORT_ID,
                    report_type: REPORT_TYPE_OUTPUT,
                },
            },
        );
        crossroads.insert(
            HID_EXTRA_INPUT_REPORT_REF_DESC_PATH,
            &[descriptor_iface],
            GattDescriptorData {
                uuid: REPORT_REFERENCE_UUID.to_string(),
                characteristic: dbus_path(HID_EXTRA_INPUT_REPORT_CHAR_PATH)?,
                flags: vec!["encrypt-read".to_string()],
                kind: DescriptorKind::ReportReference {
                    report_id: XBOX_EXTRA_INPUT_REPORT_ID,
                    report_type: REPORT_TYPE_INPUT,
                },
            },
        );
        crossroads.insert(
            HID_STATUS_INPUT_REPORT_REF_DESC_PATH,
            &[descriptor_iface],
            GattDescriptorData {
                uuid: REPORT_REFERENCE_UUID.to_string(),
                characteristic: dbus_path(HID_STATUS_INPUT_REPORT_CHAR_PATH)?,
                flags: vec!["encrypt-read".to_string()],
                kind: DescriptorKind::ReportReference {
                    report_id: XBOX_STATUS_INPUT_REPORT_ID,
                    report_type: REPORT_TYPE_INPUT,
                },
            },
        );

        crossroads.insert(
            ADVERTISEMENT_PATH,
            &[advertisement_iface],
            AdvertisementData {
                local_name: cfg.device.name.clone(),
                service_uuids: vec![
                    HID_SERVICE_UUID.to_string(),
                    BATTERY_SERVICE_UUID.to_string(),
                    DEVICE_INFO_SERVICE_UUID.to_string(),
                ],
                appearance: HID_GAMEPAD_APPEARANCE,
            },
        );

        crossroads.insert(AGENT_PATH, &[agent_iface], ());

        let crossroads = Arc::new(Mutex::new(crossroads));
        let crossroads_for_dispatch = Arc::clone(&crossroads);
        conn.start_receive(
            MatchRule::new_method_call(),
            Box::new(move |msg, conn| {
                if crossroads_for_dispatch
                    .lock()
                    .unwrap()
                    .handle_message(msg, conn)
                    .is_err()
                {
                    eprintln!("hidd: failed to handle D-Bus method call");
                }
                true
            }),
        );

        register_gatt_application(&conn, &adapter_path, &app_path)?;
        register_advertisement(&conn, &adapter_path, &advertisement_path)?;
        register_agent(&conn, &agent_path)?;

        Ok(Self {
            conn,
            state,
            input_report_char_paths,
            adapter_path,
        })
    }

    pub fn adapter_path(&self) -> &Path<'static> {
        &self.adapter_path
    }

    pub fn publish_input_report(&self, report: &[u8]) -> Result<()> {
        let (report_id, ble_payload) = ble_input_payload_from_uhid(report)?;
        let notifying = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("failed to lock HOG state"))?;
            let slot = state
                .input_reports
                .get_mut(&report_id)
                .ok_or_else(|| anyhow!("unsupported input report id=0x{report_id:02x}"))?;
            slot.value.clear();
            slot.value.extend_from_slice(ble_payload);
            slot.notifying
        };

        if !notifying {
            return self.process_pending_messages();
        }

        let input_report_char_path =
            self.input_report_char_paths
                .get(&report_id)
                .ok_or_else(|| {
                    anyhow!("missing characteristic path for report id=0x{report_id:02x}")
                })?;

        let mut changed: PropMap = HashMap::new();
        changed.insert(
            "Value".to_string(),
            Variant(Box::new(ble_payload.to_vec()) as Box<dyn RefArg>),
        );

        let signal = PropertiesPropertiesChanged {
            interface_name: GATT_CHARACTERISTIC_IFACE.to_string(),
            changed_properties: changed,
            invalidated_properties: Vec::new(),
        };

        self.conn
            .send(signal.to_emit_message(input_report_char_path))
            .map_err(|_| anyhow!("failed to emit input report notification"))?;
        self.process_pending_messages()?;
        Ok(())
    }

    fn process_pending_messages(&self) -> Result<()> {
        for _ in 0..8 {
            let had_message = self
                .conn
                .process(Duration::from_millis(0))
                .map_err(|e| anyhow!("failed to process D-Bus traffic: {e}"))?;
            if !had_message {
                break;
            }
        }
        Ok(())
    }
}

impl Drop for HogRuntime {
    fn drop(&mut self) {
        if let Ok(agent_path) = dbus_path(AGENT_PATH) {
            let _ = unregister_agent(&self.conn, &agent_path);
        }
        if let Ok(advertisement_path) = dbus_path(ADVERTISEMENT_PATH) {
            let _ = unregister_advertisement(&self.conn, &self.adapter_path, &advertisement_path);
        }
        if let Ok(app_path) = dbus_path(APP_PATH) {
            let _ = unregister_gatt_application(&self.conn, &self.adapter_path, &app_path);
        }
    }
}

fn find_adapter_path(conn: &SyncConnection) -> Result<Path<'static>> {
    let proxy = conn.with_proxy(BLUEZ_SERVICE, BLUEZ_ROOT_PATH, Duration::from_secs(10));
    let objects: HashMap<Path<'static>, HashMap<String, PropMap>> = proxy
        .get_managed_objects()
        .map_err(|e| anyhow!("GetManagedObjects on org.bluez failed: {e}"))?;

    let hci0_path = dbus_path("/org/bluez/hci0")?;
    if let Some(ifaces) = objects.get(&hci0_path) {
        if ifaces.contains_key(BLUEZ_GATT_MANAGER_IFACE)
            && ifaces.contains_key(BLUEZ_LE_ADV_MANAGER_IFACE)
        {
            return Ok(hci0_path);
        }
    }

    for (path, ifaces) in objects {
        if ifaces.contains_key(BLUEZ_GATT_MANAGER_IFACE)
            && ifaces.contains_key(BLUEZ_LE_ADV_MANAGER_IFACE)
        {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "no BlueZ adapter exposes both {BLUEZ_GATT_MANAGER_IFACE} and {BLUEZ_LE_ADV_MANAGER_IFACE}"
    ))
}

fn configure_adapter_for_hog(
    conn: &SyncConnection,
    adapter_path: &Path<'static>,
    cfg: &HidConfig,
) -> Result<()> {
    set_adapter_property(conn, adapter_path, "Powered", Variant(true))?;
    set_adapter_property(conn, adapter_path, "Pairable", Variant(true))?;
    set_adapter_property(conn, adapter_path, "Discoverable", Variant(true))?;
    if let Err(err) = set_adapter_property(conn, adapter_path, "DiscoverableTimeout", Variant(0u32))
    {
        eprintln!("hidd: unable to set adapter DiscoverableTimeout=0: {err}");
    }
    if let Err(err) = set_adapter_property(
        conn,
        adapter_path,
        "Alias",
        Variant(cfg.device.name.clone()),
    ) {
        eprintln!("hidd: unable to set adapter Alias: {err}");
    }
    Ok(())
}

fn set_adapter_property<T: Arg + Append + Send + 'static>(
    conn: &SyncConnection,
    adapter_path: &Path<'static>,
    property: &str,
    value: Variant<T>,
) -> Result<()> {
    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        adapter_path,
        DBUS_PROPERTIES_IFACE,
        "Set",
        (BLUEZ_ADAPTER_IFACE.to_string(), property.to_string(), value),
        Duration::from_secs(5),
    )
    .map_err(|e| anyhow!("failed to set adapter property {property}: {e}"))
}

fn register_gatt_application(
    conn: &SyncConnection,
    adapter_path: &Path<'static>,
    app_path: &Path<'static>,
) -> Result<()> {
    let _ = unregister_gatt_application(conn, adapter_path, app_path);

    let options: PropMap = HashMap::new();
    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        adapter_path,
        BLUEZ_GATT_MANAGER_IFACE,
        "RegisterApplication",
        (app_path.clone(), options),
        Duration::from_secs(15),
    )
    .map_err(|e| anyhow!("RegisterApplication failed: {e}"))
}

fn unregister_gatt_application(
    conn: &SyncConnection,
    adapter_path: &Path<'static>,
    app_path: &Path<'static>,
) -> Result<()> {
    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        adapter_path,
        BLUEZ_GATT_MANAGER_IFACE,
        "UnregisterApplication",
        (app_path.clone(),),
        Duration::from_secs(2),
    )
    .map_err(|e| anyhow!("UnregisterApplication failed: {e}"))
}

fn register_advertisement(
    conn: &SyncConnection,
    adapter_path: &Path<'static>,
    advertisement_path: &Path<'static>,
) -> Result<()> {
    let _ = unregister_advertisement(conn, adapter_path, advertisement_path);

    let options: PropMap = HashMap::new();
    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        adapter_path,
        BLUEZ_LE_ADV_MANAGER_IFACE,
        "RegisterAdvertisement",
        (advertisement_path.clone(), options),
        Duration::from_secs(15),
    )
    .map_err(|e| anyhow!("RegisterAdvertisement failed: {e}"))
}

fn unregister_advertisement(
    conn: &SyncConnection,
    adapter_path: &Path<'static>,
    advertisement_path: &Path<'static>,
) -> Result<()> {
    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        adapter_path,
        BLUEZ_LE_ADV_MANAGER_IFACE,
        "UnregisterAdvertisement",
        (advertisement_path.clone(),),
        Duration::from_secs(2),
    )
    .map_err(|e| anyhow!("UnregisterAdvertisement failed: {e}"))
}

fn call_method_with_dispatch<A: AppendAll>(
    conn: &SyncConnection,
    destination: &str,
    path: &Path<'static>,
    interface: &str,
    member: &str,
    args: A,
    timeout: Duration,
) -> Result<()> {
    let destination =
        BusName::new(destination).map_err(|e| anyhow!("invalid destination {destination}: {e}"))?;
    let interface =
        Interface::new(interface).map_err(|e| anyhow!("invalid interface {interface}: {e}"))?;
    let member = Member::new(member).map_err(|e| anyhow!("invalid method {member}: {e}"))?;

    let serial_state = Arc::new(AtomicU32::new(0));
    let reply_state: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
    let serial_state_return = Arc::clone(&serial_state);
    let reply_state_return = Arc::clone(&reply_state);
    let serial_state_error = Arc::clone(&serial_state);
    let reply_state_error = Arc::clone(&reply_state);

    let mut reply_rule = MatchRule::new();
    reply_rule.msg_type = Some(MessageType::MethodReturn);
    let reply_token = conn.start_receive(
        reply_rule,
        Box::new(move |msg, _| {
            let serial = serial_state_return.load(Ordering::Relaxed);
            if serial != 0 && msg.get_reply_serial() == Some(serial) {
                *reply_state_return.lock().expect("poisoned lock") = Some(Ok(()));
                false
            } else {
                true
            }
        }),
    );

    let mut error_rule = MatchRule::new();
    error_rule.msg_type = Some(MessageType::Error);
    let error_token = conn.start_receive(
        error_rule,
        Box::new(move |mut msg, _| {
            let serial = serial_state_error.load(Ordering::Relaxed);
            if serial != 0 && msg.get_reply_serial() == Some(serial) {
                let error_message = msg
                    .as_result()
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "unknown D-Bus error".to_string());
                *reply_state_error.lock().expect("poisoned lock") = Some(Err(error_message));
                false
            } else {
                true
            }
        }),
    );

    let mut msg = Message::method_call(&destination, path, &interface, &member);
    args.append(&mut IterAppend::new(&mut msg));

    let serial = match conn.send(msg) {
        Ok(serial) => serial,
        Err(_) => {
            let _ = conn.stop_receive(reply_token);
            let _ = conn.stop_receive(error_token);
            return Err(anyhow!(
                "failed to send D-Bus method call {}.{} on {}",
                interface,
                member,
                path
            ));
        }
    };
    serial_state.store(serial, Ordering::Relaxed);

    let deadline = Instant::now() + timeout;
    loop {
        if let Some(result) = reply_state.lock().expect("poisoned lock").clone() {
            let _ = conn.stop_receive(reply_token);
            let _ = conn.stop_receive(error_token);
            return result.map_err(|e| anyhow!(e));
        }

        let now = Instant::now();
        if now >= deadline {
            break;
        }

        let wait = std::cmp::min(
            deadline.saturating_duration_since(now),
            Duration::from_millis(100),
        );
        conn.process(wait).map_err(|e| {
            anyhow!(
                "failed to process D-Bus traffic while waiting for {}.{} on {}: {e}",
                interface,
                member,
                path
            )
        })?;
    }

    let _ = conn.stop_receive(reply_token);
    let _ = conn.stop_receive(error_token);
    Err(anyhow!(
        "timed out waiting for D-Bus reply to {}.{} on {}",
        interface,
        member,
        path
    ))
}

fn register_service_iface(cr: &mut Crossroads) -> IfaceToken<GattServiceData> {
    cr.register(GATT_SERVICE_IFACE, |b| {
        b.property::<String, _>("UUID")
            .get(|_, data: &mut GattServiceData| Ok(data.uuid.clone()));
        b.property::<bool, _>("Primary")
            .get(|_, data: &mut GattServiceData| Ok(data.primary));
        b.property::<Vec<Path<'static>>, _>("Includes")
            .get(|_, data: &mut GattServiceData| Ok(data.includes.clone()));
    })
}

fn register_characteristic_iface(cr: &mut Crossroads) -> IfaceToken<GattCharacteristicData> {
    cr.register(GATT_CHARACTERISTIC_IFACE, |b| {
        b.property::<String, _>("UUID")
            .get(|_, data: &mut GattCharacteristicData| Ok(data.uuid.clone()));
        b.property::<Path<'static>, _>("Service")
            .get(|_, data: &mut GattCharacteristicData| Ok(data.service.clone()));
        b.property::<Vec<String>, _>("Flags")
            .get(|_, data: &mut GattCharacteristicData| Ok(data.flags.clone()));
        b.property::<Vec<Path<'static>>, _>("Descriptors")
            .get(|_, data: &mut GattCharacteristicData| Ok(data.descriptors.clone()));

        b.method(
            "ReadValue",
            ("options",),
            ("value",),
            |_, data, (_options,): (PropMap,)| {
                let value = match &data.kind {
                    CharacteristicKind::ProtocolMode => {
                        let state = data
                            .state
                            .lock()
                            .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                        vec![state.protocol_mode]
                    }
                    CharacteristicKind::ReportMap(descriptor) => descriptor.clone(),
                    CharacteristicKind::HidInfo { country_code } => {
                        vec![
                            HID_INFO_BCD[0],
                            HID_INFO_BCD[1],
                            *country_code,
                            HID_FLAG_REMOTE_WAKE,
                        ]
                    }
                    CharacteristicKind::InputReport { report_id } => {
                        let state = data
                            .state
                            .lock()
                            .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                        state
                            .input_reports
                            .get(report_id)
                            .ok_or_else(|| {
                                MethodErr::failed(&format!(
                                    "missing input report slot for report_id=0x{report_id:02x}"
                                ))
                            })?
                            .value
                            .clone()
                    }
                    CharacteristicKind::OutputReport { report_id } => {
                        let state = data
                            .state
                            .lock()
                            .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                        state
                            .output_reports
                            .get(report_id)
                            .cloned()
                            .ok_or_else(|| {
                                MethodErr::failed(&format!(
                                    "missing output report slot for report_id=0x{report_id:02x}"
                                ))
                            })?
                    }
                    CharacteristicKind::BatteryLevel => {
                        let state = data
                            .state
                            .lock()
                            .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                        vec![state.battery_level]
                    }
                    CharacteristicKind::ControlPoint => {
                        return Err(bluez_not_supported("HID control point is write-only"));
                    }
                    CharacteristicKind::PnpId { value } => value.to_vec(),
                };
                Ok((value,))
            },
        );

        b.method(
            "WriteValue",
            ("value", "options"),
            (),
            |_, data, (value, _options): (Vec<u8>, PropMap)| match &data.kind {
                CharacteristicKind::ProtocolMode => {
                    if value.len() != 1 {
                        return Err(bluez_invalid_arguments("Protocol Mode requires exactly 1 byte"));
                    }
                    let mut state = data
                        .state
                        .lock()
                        .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                    state.protocol_mode = value[0];
                    Ok(())
                }
                CharacteristicKind::ControlPoint => {
                    if value.len() != 1 {
                        return Err(bluez_invalid_arguments(
                            "HID Control Point requires exactly 1 byte",
                        ));
                    }
                    let mut state = data
                        .state
                        .lock()
                        .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                    state.control_point = value[0];
                    Ok(())
                }
                CharacteristicKind::OutputReport { report_id } => {
                    let normalized = normalize_ble_output_value(*report_id, &value);
                    let mut state = data
                        .state
                        .lock()
                        .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                    state
                        .output_reports
                        .insert(*report_id, normalized.characteristic_value);
                    if let Some(parsed) = OutputReport::parse(&normalized.parser_value) {
                        eprintln!(
                            "hidd: dropped BLE output report rumble={{lt:{}, rt:{}, weak:{}, strong:{}}}",
                            parsed.left_trigger_magnitude,
                            parsed.right_trigger_magnitude,
                            parsed.weak_motor_magnitude,
                            parsed.strong_motor_magnitude
                        );
                    } else {
                        let report_id = normalized.parser_value.first().copied().unwrap_or(0);
                        eprintln!(
                            "hidd: dropped BLE output report report_id=0x{report_id:02x} size={}",
                            normalized.parser_value.len()
                        );
                    }
                    Ok(())
                }
                _ => Err(bluez_not_supported("characteristic is not writable")),
            },
        );

        b.method("StartNotify", (), (), |_, data, ()| match data.kind {
            CharacteristicKind::InputReport { report_id } => {
                let mut state = data
                    .state
                    .lock()
                    .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                if let Some(slot) = state.input_reports.get_mut(&report_id) {
                    slot.notifying = true;
                } else {
                    return Err(MethodErr::failed(&format!(
                        "missing input report slot for report_id=0x{report_id:02x}"
                    )));
                }
                eprintln!(
                    "hidd: BLE StartNotify input_report report_id=0x{report_id:02x}"
                );
                Ok(())
            }
            CharacteristicKind::BatteryLevel => {
                let mut state = data
                    .state
                    .lock()
                    .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                state.battery_notifying = true;
                eprintln!("hidd: BLE StartNotify battery_level");
                Ok(())
            }
            _ => Err(bluez_not_supported("characteristic does not support notifications")),
        });

        b.method("StopNotify", (), (), |_, data, ()| match data.kind {
            CharacteristicKind::InputReport { report_id } => {
                let mut state = data
                    .state
                    .lock()
                    .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                if let Some(slot) = state.input_reports.get_mut(&report_id) {
                    slot.notifying = false;
                } else {
                    return Err(MethodErr::failed(&format!(
                        "missing input report slot for report_id=0x{report_id:02x}"
                    )));
                }
                eprintln!(
                    "hidd: BLE StopNotify input_report report_id=0x{report_id:02x}"
                );
                Ok(())
            }
            CharacteristicKind::BatteryLevel => {
                let mut state = data
                    .state
                    .lock()
                    .map_err(|_| MethodErr::failed(&"failed to lock HOG state"))?;
                state.battery_notifying = false;
                eprintln!("hidd: BLE StopNotify battery_level");
                Ok(())
            }
            _ => Err(bluez_not_supported("characteristic does not support notifications")),
        });
    })
}

fn register_descriptor_iface(cr: &mut Crossroads) -> IfaceToken<GattDescriptorData> {
    cr.register(GATT_DESCRIPTOR_IFACE, |b| {
        b.property::<String, _>("UUID")
            .get(|_, data: &mut GattDescriptorData| Ok(data.uuid.clone()));
        b.property::<Path<'static>, _>("Characteristic")
            .get(|_, data: &mut GattDescriptorData| Ok(data.characteristic.clone()));
        b.property::<Vec<String>, _>("Flags")
            .get(|_, data: &mut GattDescriptorData| Ok(data.flags.clone()));

        b.method(
            "ReadValue",
            ("options",),
            ("value",),
            |_, data, (_options,): (PropMap,)| {
                let value = match data.kind {
                    DescriptorKind::ReportReference {
                        report_id,
                        report_type,
                    } => {
                        vec![report_id, report_type]
                    }
                };
                Ok((value,))
            },
        );

        b.method(
            "WriteValue",
            ("value", "options"),
            (),
            |_, _data, (_value, _options): (Vec<u8>, PropMap)| -> Result<(), MethodErr> {
                Err(bluez_not_supported("descriptor is not writable"))
            },
        );
    })
}

fn register_advertisement_iface(cr: &mut Crossroads) -> IfaceToken<AdvertisementData> {
    cr.register(LE_ADVERTISEMENT_IFACE, |b| {
        b.property::<String, _>("Type")
            .get(|_, _: &mut AdvertisementData| Ok("peripheral".to_string()));
        b.property::<Vec<String>, _>("ServiceUUIDs")
            .get(|_, data: &mut AdvertisementData| Ok(data.service_uuids.clone()));
        b.property::<String, _>("LocalName")
            .get(|_, data: &mut AdvertisementData| Ok(data.local_name.clone()));
        b.property::<u16, _>("Appearance")
            .get(|_, data: &mut AdvertisementData| Ok(data.appearance));
        b.method("Release", (), (), |_, _, ()| {
            eprintln!("hidd: BlueZ released LE advertisement");
            Ok(())
        });
    })
}

fn register_agent_iface(cr: &mut Crossroads) -> IfaceToken<()> {
    cr.register(BLUEZ_AGENT_IFACE, |b| {
        b.method(
            "Release",
            (),
            (),
            |_, _, ()| {
                eprintln!("hidd: BlueZ released pairing agent");
                Ok(())
            },
        );
        b.method(
            "RequestPinCode",
            ("device",),
            ("pincode",),
            |_, _, (_device,): (Path,)| -> Result<(String,), MethodErr> {
                Err(("org.bluez.Error.Rejected", "NoInputNoOutput agent").into())
            },
        );
        b.method(
            "DisplayPinCode",
            ("device", "pincode"),
            (),
            |_, _, (_device, _pincode): (Path, String)| -> Result<(), MethodErr> {
                Err(("org.bluez.Error.Rejected", "NoInputNoOutput agent").into())
            },
        );
        b.method(
            "RequestPasskey",
            ("device",),
            ("passkey",),
            |_, _, (_device,): (Path,)| -> Result<(u32,), MethodErr> {
                Err(("org.bluez.Error.Rejected", "NoInputNoOutput agent").into())
            },
        );
        b.method(
            "DisplayPasskey",
            ("device", "passkey", "entered"),
            (),
            |_, _, (_device, _passkey, _entered): (Path, u32, u16)| {
                Ok(())
            },
        );
        b.method(
            "RequestConfirmation",
            ("device", "passkey"),
            (),
            |_, _, (_device, _passkey): (Path, u32)| -> Result<(), MethodErr> {
                Err(("org.bluez.Error.Rejected", "NoInputNoOutput agent").into())
            },
        );
        b.method(
            "RequestAuthorization",
            ("device",),
            (),
            |_, _, (_device,): (Path,)| {
                eprintln!("hidd: agent authorized device");
                Ok(())
            },
        );
        b.method(
            "AuthorizeService",
            ("device", "uuid"),
            (),
            |_, _, (_device, _uuid): (Path, String)| {
                eprintln!("hidd: agent authorized service");
                Ok(())
            },
        );
        b.method(
            "Cancel",
            (),
            (),
            |_, _, ()| {
                eprintln!("hidd: agent pairing cancelled");
                Ok(())
            },
        );
    })
}

fn register_agent(
    conn: &SyncConnection,
    agent_path: &Path<'static>,
) -> Result<()> {
    let _ = unregister_agent(conn, agent_path);

    let bluez_path = dbus_path("/org/bluez")?;
    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        &bluez_path,
        BLUEZ_AGENT_MANAGER_IFACE,
        "RegisterAgent",
        (agent_path.clone(), "NoInputNoOutput".to_string()),
        Duration::from_secs(5),
    )
    .map_err(|e| anyhow!("RegisterAgent failed: {e}"))?;

    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        &bluez_path,
        BLUEZ_AGENT_MANAGER_IFACE,
        "RequestDefaultAgent",
        (agent_path.clone(),),
        Duration::from_secs(5),
    )
    .map_err(|e| anyhow!("RequestDefaultAgent failed: {e}"))?;

    Ok(())
}

fn unregister_agent(conn: &SyncConnection, agent_path: &Path<'static>) -> Result<()> {
    let bluez_path = dbus_path("/org/bluez")?;
    call_method_with_dispatch(
        conn,
        BLUEZ_SERVICE,
        &bluez_path,
        BLUEZ_AGENT_MANAGER_IFACE,
        "UnregisterAgent",
        (agent_path.clone(),),
        Duration::from_secs(2),
    )
    .map_err(|e| anyhow!("UnregisterAgent failed: {e}"))
}

fn bluez_not_supported(message: &'static str) -> MethodErr {
    ("org.bluez.Error.NotSupported", message).into()
}

fn bluez_invalid_arguments(message: &'static str) -> MethodErr {
    ("org.bluez.Error.InvalidArguments", message).into()
}

fn dbus_path(path: &str) -> Result<Path<'static>> {
    Path::new(path)
        .map(|p| p.into_static())
        .map_err(|e| anyhow!("invalid D-Bus object path {path}: {e}"))
}

fn ble_input_payload_from_uhid(report: &[u8]) -> Result<(u8, &[u8])> {
    let (report_id, payload) = report
        .split_first()
        .ok_or_else(|| anyhow!("UHID input report is empty"))?;
    if !matches!(
        *report_id,
        XBOX_INPUT_REPORT_ID | XBOX_EXTRA_INPUT_REPORT_ID | XBOX_STATUS_INPUT_REPORT_ID
    ) {
        return Err(anyhow!(
            "unexpected UHID input report id=0x{report_id:02x}; expected one of 0x{XBOX_INPUT_REPORT_ID:02x}/0x{XBOX_EXTRA_INPUT_REPORT_ID:02x}/0x{XBOX_STATUS_INPUT_REPORT_ID:02x}"
        ));
    }
    Ok((*report_id, payload))
}

struct NormalizedBleOutputValue {
    characteristic_value: Vec<u8>,
    parser_value: Vec<u8>,
}

fn normalize_ble_output_value(report_id: u8, value: &[u8]) -> NormalizedBleOutputValue {
    if value.len() == XBOX_OUTPUT_PAYLOAD_LEN {
        let mut parser_value = Vec::with_capacity(XBOX_OUTPUT_REPORT_LEN);
        parser_value.push(report_id);
        parser_value.extend_from_slice(value);
        return NormalizedBleOutputValue {
            characteristic_value: value.to_vec(),
            parser_value,
        };
    }

    if value.len() == XBOX_OUTPUT_REPORT_LEN && value[0] == report_id {
        return NormalizedBleOutputValue {
            characteristic_value: value[1..].to_vec(),
            parser_value: value.to_vec(),
        };
    }

    NormalizedBleOutputValue {
        characteristic_value: value.to_vec(),
        parser_value: value.to_vec(),
    }
}

fn encode_pnp_id(vendor_id: u16, product_id: u16, version: u16) -> [u8; 7] {
    let vendor = vendor_id.to_le_bytes();
    let product = product_id.to_le_bytes();
    let ver = version.to_le_bytes();
    [
        PNP_ID_VENDOR_SOURCE_USB,
        vendor[0],
        vendor[1],
        product[0],
        product[1],
        ver[0],
        ver[1],
    ]
}

#[cfg(test)]
mod tests {
    use super::{ble_input_payload_from_uhid, encode_pnp_id, normalize_ble_output_value};
    use common::hid::{
        XBOX_EXTRA_INPUT_REPORT_ID, XBOX_INPUT_REPORT_ID, XBOX_OUTPUT_PAYLOAD_LEN,
        XBOX_OUTPUT_REPORT_ID, XBOX_OUTPUT_REPORT_LEN, XBOX_STATUS_INPUT_REPORT_ID,
    };

    #[test]
    fn input_payload_strips_uhid_report_id_for_ble() {
        let raw = [XBOX_INPUT_REPORT_ID, 1, 2, 3];
        let (report_id, payload) =
            ble_input_payload_from_uhid(&raw).expect("input report should parse");
        assert_eq!(report_id, XBOX_INPUT_REPORT_ID);
        assert_eq!(payload, &[1, 2, 3]);
    }

    #[test]
    fn output_payload_adds_report_id_for_parser() {
        let raw = [7u8; XBOX_OUTPUT_PAYLOAD_LEN];
        let normalized = normalize_ble_output_value(XBOX_OUTPUT_REPORT_ID, &raw);
        assert_eq!(normalized.characteristic_value, raw);
        assert_eq!(normalized.parser_value.len(), XBOX_OUTPUT_REPORT_LEN);
        assert_eq!(normalized.parser_value[0], XBOX_OUTPUT_REPORT_ID);
        assert_eq!(&normalized.parser_value[1..], raw);
    }

    #[test]
    fn output_payload_with_in_band_id_is_normalized_to_characteristic_payload() {
        let mut raw = Vec::with_capacity(XBOX_OUTPUT_REPORT_LEN);
        raw.push(XBOX_OUTPUT_REPORT_ID);
        raw.extend_from_slice(&[9u8; XBOX_OUTPUT_PAYLOAD_LEN]);

        let normalized = normalize_ble_output_value(XBOX_OUTPUT_REPORT_ID, &raw);
        assert_eq!(
            normalized.characteristic_value.len(),
            XBOX_OUTPUT_PAYLOAD_LEN
        );
        assert_eq!(
            normalized.characteristic_value,
            vec![9u8; XBOX_OUTPUT_PAYLOAD_LEN]
        );
        assert_eq!(normalized.parser_value, raw);
    }

    #[test]
    fn accepts_extra_input_report_ids() {
        let raw = [XBOX_EXTRA_INPUT_REPORT_ID, 0xaa];
        let (report_id, payload) =
            ble_input_payload_from_uhid(&raw).expect("extra input report should parse");
        assert_eq!(report_id, XBOX_EXTRA_INPUT_REPORT_ID);
        assert_eq!(payload, &[0xaa]);

        let raw = [XBOX_STATUS_INPUT_REPORT_ID, 0xbb];
        let (report_id, payload) =
            ble_input_payload_from_uhid(&raw).expect("status input report should parse");
        assert_eq!(report_id, XBOX_STATUS_INPUT_REPORT_ID);
        assert_eq!(payload, &[0xbb]);
    }

    #[test]
    fn pnp_id_encoding_uses_usb_source_and_little_endian_fields() {
        let pnp = encode_pnp_id(0x045e, 0x02fd, 0x0408);
        assert_eq!(pnp, [0x02, 0x5e, 0x04, 0xfd, 0x02, 0x08, 0x04]);
    }
}
