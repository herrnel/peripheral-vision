# Peripheral-Vision

Cross-platform Rust CLI to access device peripherals across Unix, Linux, Windows, and WSL.

Current implementation:
- Fully implemented: `serial`, `hid`
- CLI contract in place for: `usb`, `ble`, `gpio`, `camera`, `audio` (returns explicit unsupported-kind errors today)

## Build

```bash
cargo build
cargo test
```

## Commands

```bash
# Platform + WSL detection
cargo run -p periph-cli -- platform

# Preflight checks (includes serial scan)
cargo run -p periph-cli -- preflight

# Scan peripherals (all currently maps to implemented kinds)
cargo run -p periph-cli -- scan --kind all
cargo run -p periph-cli -- scan --kind serial
cargo run -p periph-cli -- scan --kind hid

# Read bytes from serial
cargo run -p periph-cli -- read --port /dev/ttyUSB0 --baud 115200 --bytes 256

# Read bytes from HID by selector (vendor:product[:serial])
cargo run -p periph-cli -- read --kind hid --port 2341:0043

# Write utf8 payload
cargo run -p periph-cli -- write --port /dev/ttyUSB0 --data "hello"

# Write hex payload
cargo run -p periph-cli -- write --port /dev/ttyUSB0 --hex --data "48 65 6c 6c 6f"

# Write HID report bytes (first byte can be report ID)
cargo run -p periph-cli -- write --kind hid --port 2341:0043 --hex --data "00 01 02 03"

# Monitor serial stream
cargo run -p periph-cli -- monitor --port /dev/ttyUSB0 --baud 115200

# Monitor HID stream
cargo run -p periph-cli -- monitor --kind hid --port 2341:0043 --chunk-size 64
```

## JSON output

Add `--json` to any command for machine-readable output:

```bash
cargo run -p periph-cli -- --json scan --kind serial
```

## WSL note

Direct USB/serial in WSL may need USB passthrough with `usbipd-win`.
