use btleplug::api::{Central, CharPropFlags, Manager as _, Peripheral, ScanFilter, WriteType};
use btleplug::platform::Manager;
use futures::StreamExt;
use std::str::FromStr;
use tokio::sync::watch;
use uuid::Uuid;

use crate::bike_data::{self, BikeData};

const BIKE_NAME: &str = "ECHO_BIKE_004130";
const FTMS_CONTROL_POINT: &str = "00002ad9-0000-1000-8000-00805f9b34fb";
const INDOOR_BIKE_DATA: &str = "00002ad2-0000-1000-8000-00805f9b34fb";

pub async fn run(tx: watch::Sender<BikeData>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .next()
        .expect("No Bluetooth adapters found");

    central.start_scan(ScanFilter::default()).await?;

    // Give scanning a moment to find devices
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let peripherals = central.peripherals().await?;

    for peripheral in &peripherals {
        let props = match peripheral.properties().await? {
            Some(p) => p,
            None => continue,
        };
        let name = match &props.local_name {
            Some(n) => n.clone(),
            None => continue,
        };

        if name != BIKE_NAME {
            continue;
        }

        println!("Found {BIKE_NAME}, connecting...");
        peripheral.connect().await?;
        println!("Connected to bike");

        peripheral.discover_services().await?;
        println!("Services discovered");

        let chars = peripheral.characteristics();

        // Write FTMS Control Point: request control then start
        let cp_uuid = Uuid::from_str(FTMS_CONTROL_POINT)?;
        for ch in &chars {
            if ch.uuid == cp_uuid {
                peripheral.write(ch, &[0x00], WriteType::WithResponse).await?;
                peripheral.write(ch, &[0x07], WriteType::WithResponse).await?;
                println!("FTMS control point initialized");
                break;
            }
        }

        // Subscribe to Indoor Bike Data
        let ibd_uuid = Uuid::from_str(INDOOR_BIKE_DATA)?;
        for ch in &chars {
            if ch.uuid == ibd_uuid && ch.properties.contains(CharPropFlags::NOTIFY) {
                println!("Subscribing to Indoor Bike Data...");
                peripheral.subscribe(ch).await?;

                let mut notifications = peripheral.notifications().await?;

                while let Some(notif) = notifications.next().await {
                    if let Some(bike) = bike_data::parse_indoor_bike_data(&notif.value) {
                        let time_str = bike
                            .elapsed_secs
                            .map(|s| format!("{:02}:{:02}", s / 60, s % 60))
                            .unwrap_or_else(|| "--:--".to_string());

                        println!(
                            "speed: {:.1} km/h | cad: {:.0} rpm | watts: {} | cals: {} | hr: {} | time: {}",
                            bike.speed_kmh.unwrap_or(0.0),
                            bike.cadence_rpm.unwrap_or(0.0),
                            bike.power_watts.map(|v| v.to_string()).unwrap_or("-".into()),
                            bike.calories.map(|v| v.to_string()).unwrap_or("-".into()),
                            bike.heart_rate.map(|v| v.to_string()).unwrap_or("-".into()),
                            time_str,
                        );

                        let _ = tx.send(bike);
                    }
                }
                break;
            }
        }

        peripheral.disconnect().await?;
        println!("Disconnected from bike");
        break;
    }

    Ok(())
}
