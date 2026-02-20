# Echo2Garmin

BLE bridge that reads data from a Rogue Echo Bike and re-broadcasts it as a standard Cycling Power sensor, so a Garmin watch can pick it up during an Indoor Cycling activity.

```
Echo Bike --FTMS--> PC --Cycling Power Service--> Garmin Watch
```

## How it works

- **Central role** (btleplug): Connects to the Echo Bike, initializes FTMS, and subscribes to Indoor Bike Data notifications
- **Peripheral role** (bluer): Registers a GATT server with Cycling Power Service (0x1818) and advertises as "Echo2Garmin"
- Bike data (power, cadence) is parsed and re-encoded as Cycling Power Measurement packets in real-time

## Requirements

- Linux with BlueZ
- Bluetooth adapter that supports both central and peripheral roles
- Nix (for dev environment) or Rust toolchain with `dbus` and `openssl` dev libraries

## Usage

1. Power on the Echo Bike and wake the display
2. Run the bridge:
   ```bash
   nix develop --command cargo run
   ```
3. Wait for `BLE peripheral advertising as Echo2Garmin` and data lines to appear
4. On your Garmin watch: start an Indoor Cycling activity, go to Settings > Sensors & Accessories > Add New > Power Meter, and select "Echo2Garmin"

## Project structure

```
src/
  main.rs          Orchestration
  bike_data.rs     Shared types, FTMS parser, Cycling Power encoder
  central.rs       Echo Bike connection via btleplug
  peripheral.rs    GATT server + BLE advertising via bluer
```
