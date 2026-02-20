use uuid::Uuid;

// FTMS
pub const FTMS_SERVICE: Uuid = Uuid::from_u128(0x00001826_0000_1000_8000_00805f9b34fb);
pub const INDOOR_BIKE_DATA: Uuid = Uuid::from_u128(0x00002ad2_0000_1000_8000_00805f9b34fb);
pub const FTMS_CONTROL_POINT: Uuid = Uuid::from_u128(0x00002ad9_0000_1000_8000_00805f9b34fb);

// Cycling Power Service
pub const CYCLING_POWER_SERVICE: Uuid = Uuid::from_u128(0x00001818_0000_1000_8000_00805f9b34fb);
pub const CYCLING_POWER_MEASUREMENT: Uuid = Uuid::from_u128(0x00002a63_0000_1000_8000_00805f9b34fb);
pub const CYCLING_POWER_FEATURE: Uuid = Uuid::from_u128(0x00002a65_0000_1000_8000_00805f9b34fb);
pub const SENSOR_LOCATION: Uuid = Uuid::from_u128(0x00002a5d_0000_1000_8000_00805f9b34fb);

#[derive(Clone, Debug, Default)]
pub struct BikeData {
    pub power_watts: Option<i16>,
    pub cadence_rpm: Option<f32>,
    pub heart_rate: Option<u8>,
    pub speed_kmh: Option<f32>,
    pub distance_m: Option<f32>,
    pub calories: Option<u16>,
    pub elapsed_secs: Option<u16>,
}

/// Encode Cycling Power Measurement (0x2A63).
///
/// Layout:
///   Flags (u16) | Power (i16) | Cumulative Crank Revolutions (u16) | Last Crank Event Time (u16)
///
/// `crank_revs` and `crank_event_time` are wrapping counters maintained by the caller.
pub fn encode_cycling_power(
    power: i16,
    crank_revs: u16,
    crank_event_time: u16,
) -> Vec<u8> {
    let flags: u16 = 1 << 5; // bit 5 = crank revolution data present
    let mut buf = Vec::with_capacity(8);
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&power.to_le_bytes());
    buf.extend_from_slice(&crank_revs.to_le_bytes());
    buf.extend_from_slice(&crank_event_time.to_le_bytes());
    buf
}

/// Parse Indoor Bike Data (0x2AD2) from FTMS notification payload.
pub fn parse_indoor_bike_data(data: &[u8]) -> Option<BikeData> {
    if data.len() < 2 {
        return None;
    }

    let flags = u16::from_le_bytes([data[0], data[1]]);
    let mut offset = 2;
    let mut bike = BikeData::default();

    // Bit 0: More Data — 0 means Instantaneous Speed IS present
    if flags & (1 << 0) == 0 {
        let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        bike.speed_kmh = Some(v as f32 * 0.01);
    }

    // Bit 1: Average Speed
    if flags & (1 << 1) != 0 {
        offset += 2;
    }

    // Bit 2: Instantaneous Cadence
    if flags & (1 << 2) != 0 {
        let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        bike.cadence_rpm = Some(v as f32 * 0.5);
    }

    // Bit 3: Average Cadence
    if flags & (1 << 3) != 0 {
        offset += 2;
    }

    // Bit 4: Total Distance (uint24, meters)
    if flags & (1 << 4) != 0 {
        let v = (data[offset] as u32)
            | ((data[offset + 1] as u32) << 8)
            | ((data[offset + 2] as u32) << 16);
        offset += 3;
        bike.distance_m = Some(v as f32);
    }

    // Bit 5: Resistance Level
    if flags & (1 << 5) != 0 {
        offset += 2;
    }

    // Bit 6: Instantaneous Power
    if flags & (1 << 6) != 0 {
        let v = i16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        bike.power_watts = Some(v);
    }

    // Bit 7: Average Power
    if flags & (1 << 7) != 0 {
        offset += 2;
    }

    // Bit 8: Expended Energy (total u16 + per_hour u16 + per_minute u8)
    if flags & (1 << 8) != 0 {
        let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 5;
        bike.calories = Some(v);
    }

    // Bit 9: Heart Rate
    if flags & (1 << 9) != 0 {
        bike.heart_rate = Some(data[offset]);
        offset += 1;
    }

    // Bit 10: Metabolic Equivalent
    if flags & (1 << 10) != 0 {
        offset += 1;
    }

    // Bit 11: Elapsed Time
    if flags & (1 << 11) != 0 {
        let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        bike.elapsed_secs = Some(v);
    }

    // Bit 12: Remaining Time
    if flags & (1 << 12) != 0 {
        offset += 2;
    }

    let _ = offset;
    Some(bike)
}
