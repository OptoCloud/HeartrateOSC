// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use btleplug::api::Characteristic;
use btleplug::api::{bleuuid::uuid_from_u16, Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use futures::stream::StreamExt;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tokio::main]
async fn main() {
    let ble_thread_handle = tokio::spawn(async move {ble_thread().await});

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    ble_thread_handle.await.unwrap();
}

async fn ble_thread() {
    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await;

    // Vector of threads for each adapter
    let mut adapter_threads = Vec::<>::new();

    for adapter in adapters.unwrap() {
        let adapter_thread = tokio::spawn(async move {ble_adapter_thread(&adapter).await});

        adapter_threads.push(adapter_thread);
    }

    for adapter_thread in adapter_threads {
        adapter_thread.await.unwrap();
    }
}

async fn ble_adapter_thread(adapter: &Adapter) -> Result<(), btleplug::Error> {
    let mut events = adapter.events().await?;

    adapter.start_scan(ScanFilter::default()).await?;

    let mut device_id: Option<PeripheralId> = None;

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                let peripheral = adapter.peripheral(&id).await?;
                let characteristic = get_polar_h10_peripheral_characteristic(&peripheral).await?;
                if characteristic.is_none() {
                    continue;
                }

                adapter.stop_scan().await?;

                device_id = Some(id);
            }
            CentralEvent::DeviceConnected(id) => {
                if device_id.is_none() || id != *device_id.as_ref().unwrap() {
                    continue;
                }

                println!("DeviceConnected: {:?}", id);
            }
            CentralEvent::DeviceDisconnected(id) => {
                device_id = None;
                println!("DeviceDisconnected: {:?}", id);
            }
            _ => {}
        }
    }

    Ok(())
}

const HEART_RATE_SERVICE_UUID: u16 = 0x180D;
const HEART_RATE_MEASUREMENT_CHARACTERISTIC_UUID: u16 = 0x2A37;
const POLAR_ELECTRO_OY_MANUFACTURER_ID: u16 = 0x6B;
const POLAR_ELECTRO_EU_MANUFACTURER_ID: u16 = 0xD1;

async fn get_polar_h10_peripheral_characteristic(peripheral: &Peripheral) -> Result<Option<Characteristic>, btleplug::Error> {
    let props = peripheral.properties().await.unwrap_or(None);
    if props.is_none() {
        return Ok(None);
    }

    let props = props.unwrap();

    // Check if the peripheral is made by Polar
    if props.manufacturer_data.iter().any(|(k, _)| *k == POLAR_ELECTRO_OY_MANUFACTURER_ID || *k == POLAR_ELECTRO_EU_MANUFACTURER_ID) {
        println!("Found Polar device");

        peripheral.connect().await?;

        peripheral.discover_services().await?;

        // Check if the peripheral has a Heart Rate service
        let services = peripheral.services();

        for service in services {
            if service.uuid != uuid_from_u16(HEART_RATE_SERVICE_UUID) {
                continue;
            }
            
            println!("Found Heart Rate service!");

            let characteristics = peripheral.characteristics();

            for characteristic in characteristics {
                if characteristic.uuid != uuid_from_u16(HEART_RATE_MEASUREMENT_CHARACTERISTIC_UUID) {
                    continue;
                }
                
                println!("Found Heart Rate Measurement characteristic!");

                peripheral.subscribe(&characteristic).await?;
                
                let mut notification_stream = peripheral.notifications().await?;
                
                while let Some(data) = notification_stream.next().await {
                    println!("Possible Heart Rate: {:?} BPM, full data: {:?}", data.value[1], data.value);

                }

                return Ok(Some(characteristic));
            }
        }
    }
    
    return Ok(None);
}