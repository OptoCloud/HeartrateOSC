// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::AtomicBool;
use btleplug::api::Characteristic;
use btleplug::api::{bleuuid::uuid_from_u16, Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager as BleManager, Peripheral, PeripheralId};
use futures::stream::StreamExt;
use tauri::Manager;
use std::collections::{HashMap, HashSet};

mod appsettings;
use appsettings::AppSettings;

mod heartrate_measurement;
use heartrate_measurement::HeartRateMeasurement;

mod vrcosc_client;
use vrcosc_client::VRCOSCClient;

const APP_NAME: &str = "PulseOSC";

static APP_SETTINGS: std::sync::Mutex<Option<AppSettings>> = std::sync::Mutex::new(None);

#[tauri::command]
fn bluetooth_init(app_handle: tauri::AppHandle) {
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    
    if INITIALIZED.fetch_or(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    tokio::spawn(async move {
        bluetooth_adapters_watch(app_handle).await;
    });
}

async fn bluetooth_adapters_watch(app_handle: tauri::AppHandle) {
    let manager = BleManager::new().await.unwrap();

    let mut running_tasks = Vec::<(tokio::task::JoinHandle<()>, String)>::new();
    let mut watched_adapters = HashSet::<String>::new();

    loop {
        let adapters = manager.adapters().await.unwrap();

        for adapter in adapters {
            let adapter_info = adapter.adapter_info().await.unwrap();

            if watched_adapters.contains(&adapter_info) {
                continue;
            }

            println!("Found adapter: {:?}", adapter_info);

            let app_handle = app_handle.clone();

            let task = tokio::spawn(async move {
                handle_bt_adapter(adapter, app_handle).await.unwrap();
            });

            running_tasks.push((task, adapter_info.clone()));
            watched_adapters.insert(adapter_info);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        running_tasks.retain(|task| !task.0.is_finished());
    }
}

async fn handle_bt_adapter(adapter: Adapter, app_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mut events = adapter.events().await?;

    let mut peripherals = HashMap::<PeripheralId, Peripheral>::new();

    loop {
        println!("Scanning for devices...");
        adapter.start_scan(ScanFilter::default()).await?;

        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral = adapter.peripheral(&id).await?;
                    if !is_compatible_manufacturer(&peripheral).await {
                        continue;
                    }

                    println!("Discovered device from compatible manufacturer: {:?}", id);

                    peripheral.connect().await?;
                }
                CentralEvent::DeviceConnected(id) => {
                    let peripheral = adapter.peripheral(&id).await?;
                    if !has_heartrate_service(&peripheral).await? {
                        peripheral.disconnect().await?;
                        continue;
                    }

                    println!("Connected to device with Heart Rate service: {:?}", id);

                    let heartrate_measurement_characteristic = get_heartrate_measurement_characteristic(&peripheral);
                    if heartrate_measurement_characteristic.is_none() {
                        peripheral.disconnect().await?;
                        continue;
                    }
                    let heartrate_measurement_characteristic = heartrate_measurement_characteristic.unwrap();

                    println!("Found Heart Rate Measurement characteristic!");

                    peripherals.insert(id, peripheral.clone());

                    handle_device(&peripheral, &heartrate_measurement_characteristic, app_handle.clone()).await?;
                }
                CentralEvent::DeviceDisconnected(id) => {
                    peripherals.remove(&id);
                    println!("DeviceDisconnected: {:?}", id);
                }
                _ => {}
            }
        }
    }
} 

const HEART_RATE_SERVICE_UUID: u16 = 0x180D;
const HEART_RATE_MEASUREMENT_CHARACTERISTIC_UUID: u16 = 0x2A37;
const POLAR_ELECTRO_OY_MANUFACTURER_ID: u16 = 0x6B;
const POLAR_ELECTRO_EU_MANUFACTURER_ID: u16 = 0xD1;

async fn is_compatible_manufacturer(peripheral: &Peripheral) -> bool {
    let props = peripheral.properties().await.unwrap_or(None);
    if props.is_none() {
        return false;
    }

    let props = props.unwrap();

    // Check if the peripheral is made by Polar
    if props.manufacturer_data.iter().any(|(k, _)| *k == POLAR_ELECTRO_OY_MANUFACTURER_ID || *k == POLAR_ELECTRO_EU_MANUFACTURER_ID) {
        println!("Found Polar device");

        return true;
    }
    
    false
}

async fn has_heartrate_service(peripheral: &Peripheral) -> Result<bool, btleplug::Error> {
    peripheral.discover_services().await?;

    let services = peripheral.services();

    for service in services {
        if service.uuid == uuid_from_u16(HEART_RATE_SERVICE_UUID) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn get_heartrate_measurement_characteristic(peripheral: &Peripheral) -> Option<Characteristic> {
    let characteristics = peripheral.characteristics();

    for characteristic in characteristics {
        if characteristic.uuid != uuid_from_u16(HEART_RATE_MEASUREMENT_CHARACTERISTIC_UUID) {
            continue;
        }
        
        return Some(characteristic);
    }

    None
}

async fn handle_device(peripheral: &Peripheral, characteristic: &Characteristic, app_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    peripheral.subscribe(&characteristic).await?;
    
    let mut notification_stream = peripheral.notifications().await?;

    let from_lock = {
        let app_settings = APP_SETTINGS.lock().unwrap();
        (*app_settings).clone().expect("App settings not set")
    };

    let ipv4 = std::net::Ipv4Addr::new(127, 0, 0, 1);
    let addr = std::net::SocketAddr::new(ipv4.into(), from_lock.osc_port);
    let mut vrcosc_client = VRCOSCClient::new(&addr);
    
    while let Some(data) = notification_stream.next().await {
        let flags = data.value[0];
        let heart_rate_format = flags & 0b00000001 != 0;
        let sensor_contact_detected = flags & 0b00000010 != 0;
        let sensor_contact_supported = flags & 0b00000100 != 0;
        let energy_expended_present = flags & 0b00001000 != 0;
        let rr_interval_present = flags & 0b00010000 != 0;
        let heart_rate: u16;
        let energy_expended: u16;
        let mut rr_intervals = Vec::<u16>::new();

        let mut offset = 1;
        
        if heart_rate_format {
            // Heart Rate is in 8-bit format
            heart_rate = u16::from_le_bytes([data.value[1], data.value[2]]);
            offset += 2;
        } else {
            // Heart Rate is in 16-bit format
            heart_rate = u16::from_le_bytes([data.value[1], 0]);
            offset += 1;
        }

        if energy_expended_present {
            println!("Energy Expended present");
            energy_expended = u16::from_le_bytes([data.value[offset], data.value[offset + 1]]);
            offset += 2;
        } else {
            energy_expended = 0;
        }

        if rr_interval_present {
            // Loop through all RR Intervals
            while offset + 1 < data.value.len() {
                let rr_interval = u16::from_le_bytes([data.value[offset], data.value[offset + 1]]);
                rr_intervals.push(rr_interval);
                offset += 2;
            }
        }

        let mesage = HeartRateMeasurement {
            heart_rate,
            sensor_contact_detected,
            sensor_contact_supported,
            energy_expended_present,
            energy_expended,
            rr_intervals
        };
        
        vrcosc_client.send_heartrate(&mesage)?;

        app_handle.emit_all("heartRateMeasurement", mesage)?;
    }

    Ok(())
}

fn get_app_config_path() -> std::path::PathBuf {
    let dir = dirs::data_local_dir().expect("Failed to get local app data directory").join(APP_NAME);

    if !dir.exists() {
        std::fs::create_dir_all(&dir).expect("Failed to create app data directory");
    }

    dir
}

fn get_app_settings_path() -> std::path::PathBuf {
    get_app_config_path().join("settings.json")
}

fn write_app_settings(settings: &AppSettings) {
    let settings_path = get_app_settings_path();

    let settings_json = serde_json::to_string_pretty(&settings).expect("Failed to serialize settings");

    std::fs::write(&settings_path, settings_json).expect("Failed to write settings file");
}

fn reset_app_settings() {
    let app_settings = AppSettings {
        osc_adress: "127.0.0.1".to_string(),
        osc_port: 9000
    };

    write_app_settings(&app_settings);

    APP_SETTINGS.lock().unwrap().replace(app_settings.clone());
}

fn read_app_settings() {
    let settings_path = get_app_settings_path();

    if !settings_path.exists() {
        reset_app_settings();
        return;
    }

    let settings_file = std::fs::read_to_string(&settings_path);
    if settings_file.is_err() {
        reset_app_settings();
        return;
    }
    let settings_file = settings_file.unwrap();

    let settings = serde_json::from_str::<AppSettings>(&settings_file);
    if settings.is_err() {
        reset_app_settings();
        return;
    }

    APP_SETTINGS.lock().unwrap().replace(settings.unwrap());
}

#[tokio::main]
async fn main() {
    read_app_settings();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![bluetooth_init])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}