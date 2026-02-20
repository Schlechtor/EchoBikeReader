use bluer::adv::Advertisement;
use bluer::gatt::local::{
    characteristic_control, Application, Characteristic, CharacteristicControlEvent,
    CharacteristicNotify, CharacteristicNotifyMethod, CharacteristicRead, Service,
};
use bluer::gatt::CharacteristicWriter;
use bluer::Adapter;
use futures::{pin_mut, StreamExt};
use tokio::sync::watch;

use crate::bike_data::{self, BikeData};

pub async fn run(adapter: &Adapter, mut rx: watch::Receiver<BikeData>) -> bluer::Result<()> {
    // Crank revolution counters (wrapping)
    let mut crank_revs: u16 = 0;
    let mut crank_event_time: u16 = 0;

    // Set up characteristic controls
    let (power_control, power_handle) = characteristic_control();

    // Cycling Power Feature: read-only, returns feature flags
    // Bit 1 = Crank Revolution Data Supported
    let power_feature_value: u32 = 1 << 1;
    let power_feature_bytes = power_feature_value.to_le_bytes().to_vec();

    // Sensor Location: read-only, 0x05 = Left Crank
    let sensor_location_bytes = vec![0x05u8];

    let app = Application {
        services: vec![Service {
            uuid: bike_data::CYCLING_POWER_SERVICE,
            primary: true,
            characteristics: vec![
                Characteristic {
                    uuid: bike_data::CYCLING_POWER_MEASUREMENT,
                    notify: Some(CharacteristicNotify {
                        notify: true,
                        method: CharacteristicNotifyMethod::Io,
                        ..Default::default()
                    }),
                    control_handle: power_handle,
                    ..Default::default()
                },
                Characteristic {
                    uuid: bike_data::CYCLING_POWER_FEATURE,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |_req| {
                            let bytes = power_feature_bytes.clone();
                            Box::pin(async move { Ok(bytes) })
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: bike_data::SENSOR_LOCATION,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |_req| {
                            let bytes = sensor_location_bytes.clone();
                            Box::pin(async move { Ok(bytes) })
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };

    let _app_handle = adapter.serve_gatt_application(app).await?;
    println!("GATT server registered");

    // Advertise
    let adv = Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        service_uuids: vec![bike_data::CYCLING_POWER_SERVICE].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("Echo2Garmin".to_string()),
        ..Default::default()
    };
    let _adv_handle = adapter.advertise(adv).await?;
    println!("BLE peripheral advertising as Echo2Garmin");

    let mut power_writer: Option<CharacteristicWriter> = None;

    pin_mut!(power_control);

    loop {
        tokio::select! {
            evt = power_control.next() => {
                match evt {
                    Some(CharacteristicControlEvent::Notify(writer)) => {
                        println!("Garmin subscribed to Cycling Power");
                        power_writer = Some(writer);
                    }
                    None => break,
                    _ => {}
                }
            }
            res = rx.changed() => {
                if res.is_err() {
                    break; // sender dropped
                }
                let bike = rx.borrow_and_update().clone();

                let cadence = bike.cadence_rpm.unwrap_or(0.0);
                let power = bike.power_watts.unwrap_or(0);

                // Advance crank counters
                if cadence > 0.0 {
                    crank_revs = crank_revs.wrapping_add(1);
                    let tick = (1024.0 * 60.0 / cadence) as u16;
                    crank_event_time = crank_event_time.wrapping_add(tick);
                }

                let payload = bike_data::encode_cycling_power(power, crank_revs, crank_event_time);

                if let Some(writer) = power_writer.as_mut() {
                    use tokio::io::AsyncWriteExt;
                    if let Err(err) = writer.write_all(&payload).await {
                        eprintln!("Notify error: {err}");
                        power_writer = None;
                    }
                }
            }
        }
    }

    Ok(())
}
