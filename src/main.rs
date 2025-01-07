use btleplug::api::{Central, CharPropFlags, Manager as _, Peripheral, ScanFilter, WriteType};
use btleplug::platform::Manager;
use std::str::FromStr;
use futures::stream::StreamExt;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), btleplug::Error> {
    let manager = Manager::new().await?;

    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .nth(0)
        .expect("No Bluetooth adapters found");

    central.start_scan(ScanFilter::default()).await?;

    let peripherals = central.peripherals().await?;

    for peripheral in peripherals.iter() {
        println!("{:?}", peripheral.properties().await?.unwrap().local_name);

        if peripheral.properties().await?.unwrap().local_name.is_some() {
            if peripheral.properties().await?.unwrap().local_name.unwrap() == "ECHO_BIKE_004130" {
                println!("found you");

                let properties = peripheral.properties().await?;
                let local_name = properties
                .unwrap()
                .local_name
                .unwrap_or(String::from("(peripheral name unknown)"));

                println!("Connecting to peripheral {:?}...", &local_name);
                if let Err(err) = peripheral.connect().await {
                    eprintln!("Error connecting to peripheral, skipping: {}", err);
                    continue;
                }

                let is_connected = peripheral.is_connected().await?;
                println!(
                    "Now connected ({:?}) to peripheral {:?}...",
                    is_connected, local_name
                );

                peripheral.discover_services().await?;
                println!("Discover peripheral {:?} services...", &local_name);

                for characteristic in peripheral.characteristics() {
                    if characteristic.uuid == Uuid::from_str("00002ad9-0000-1000-8000-00805f9b34fb").unwrap() {
                        println!("Writing");
                        peripheral.write(&characteristic, &vec![0x00], WriteType::WithResponse).await?;
                        peripheral.write(&characteristic, &vec![0x07], WriteType::WithResponse).await?;
                    }
                }

                for characteristic in peripheral.characteristics() {
                    if characteristic.uuid == Uuid::from_str("00002ad2-0000-1000-8000-00805f9b34fb").unwrap() 
                        && characteristic.properties.contains(CharPropFlags::NOTIFY)
                    {
                        println!("Subscribing to characteristic {:?}", characteristic.uuid);
                        peripheral.subscribe(&characteristic).await?;

                        let mut notification_stream =
                            peripheral.notifications().await?;

                        while let Some(x) = notification_stream.next().await {
                            println!("Received data from {:?} {:?}", local_name,  x.value);

                            let data = x.value;
                            let speed = u16::from_le_bytes([data[2], data[3]]) as f32 * 0.01;
                            // let avg_speed = u16::from_le_bytes([data[4], data[5]]) as f32 * 0.01;
                            let cadence = u16::from_le_bytes([data[6], data[7]]) as f32 * 0.5;
                            let dist = (data[10] as u32) | ((data[11] as u32) << 8) | ((data[12] as u32) << 16);
                            let d = dist as f32 * 0.0006213712;
                            let power = i16::from_le_bytes([data[13], data[14]]);                        
                            let time = u16::from_le_bytes([data[19], data[20]]);

                            println!("speed: {:?}, cad: {:?}, dist: {:?}, watts: {:?}, time: {:?}", speed, cadence, d, power, time);
                            tokio::signal::ctrl_c().await.expect("failed to listen for event");
                        }
                    }
                }
                println!("Disconnecting from peripheral {:?}...", local_name);
                peripheral.disconnect().await?;
            }
        }
    }
    Ok(())
}
