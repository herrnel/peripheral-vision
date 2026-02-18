use crate::error::{PeriphError, Result};
use crate::peripheral::{
    DeviceInfo, MonitorRequest, PeripheralKind, ReadRequest, ReadResponse, WriteRequest,
    WriteResponse,
};
use glob::glob;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

pub fn scan_serial_ports() -> Result<Vec<DeviceInfo>> {
    #[cfg(windows)]
    {
        scan_windows_ports()
    }

    #[cfg(not(windows))]
    {
        scan_unix_ports()
    }
}

pub fn read_from_serial(request: &ReadRequest) -> Result<ReadResponse> {
    let mut port = open_serial_port(&request.port, request.baud_rate, request.timeout_ms)?;
    let mut buffer = vec![0_u8; request.max_bytes];

    let bytes_read = match port.read(buffer.as_mut_slice()) {
        Ok(count) => count,
        Err(err) if err.kind() == ErrorKind::TimedOut => 0,
        Err(err) => return Err(err.into()),
    };

    buffer.truncate(bytes_read);
    Ok(ReadResponse {
        bytes_read,
        data_hex: hex_encode(&buffer),
        data_utf8: String::from_utf8(buffer).ok(),
    })
}

pub fn write_to_serial(request: &WriteRequest) -> Result<WriteResponse> {
    let mut port = open_serial_port(&request.port, request.baud_rate, request.timeout_ms)?;
    port.write_all(&request.data)?;
    port.flush()?;

    Ok(WriteResponse {
        bytes_written: request.data.len(),
    })
}

pub fn monitor_serial<F, S>(request: &MonitorRequest, mut on_chunk: F, should_stop: S) -> Result<()>
where
    F: FnMut(ReadResponse),
    S: Fn() -> bool,
{
    let mut port = open_serial_port(&request.port, request.baud_rate, request.timeout_ms)?;
    let mut buffer = vec![0_u8; request.chunk_size.max(1)];
    let start = Instant::now();

    loop {
        if should_stop() {
            break;
        }

        if let Some(limit) = request.duration_seconds {
            if start.elapsed() >= Duration::from_secs(limit) {
                break;
            }
        }

        match port.read(buffer.as_mut_slice()) {
            Ok(0) => {}
            Ok(count) => {
                let chunk = &buffer[..count];
                on_chunk(ReadResponse {
                    bytes_read: count,
                    data_hex: hex_encode(chunk),
                    data_utf8: String::from_utf8(chunk.to_vec()).ok(),
                });
            }
            Err(err) if err.kind() == ErrorKind::TimedOut => {}
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}

fn open_serial_port(
    port_name: &str,
    baud_rate: u32,
    timeout_ms: u64,
) -> Result<Box<dyn serialport::SerialPort>> {
    serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(Into::into)
}

#[cfg(not(windows))]
fn scan_unix_ports() -> Result<Vec<DeviceInfo>> {
    let mut found = BTreeSet::new();

    for pattern in unix_patterns() {
        let entries = glob(pattern)
            .map_err(|err| PeriphError::InvalidInput(format!("invalid glob pattern: {err}")))?;
        for path in entries.flatten() {
            if path.exists() {
                found.insert(path);
            }
        }
    }

    Ok(found
        .into_iter()
        .map(|path| {
            let label = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            let path_str = path.display().to_string();

            DeviceInfo {
                id: path_str.clone(),
                kind: PeripheralKind::Serial,
                label,
                path: Some(path_str),
                metadata: BTreeMap::new(),
            }
        })
        .collect())
}

#[cfg(not(windows))]
fn unix_patterns() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    {
        &["/dev/tty.*", "/dev/cu.*"]
    }

    #[cfg(not(target_os = "macos"))]
    {
        &[
            "/dev/ttyUSB*",
            "/dev/ttyACM*",
            "/dev/ttyS*",
            "/dev/ttyAMA*",
            "/dev/rfcomm*",
            "/dev/tty.*",
            "/dev/cu.*",
        ]
    }
}

#[cfg(windows)]
fn scan_windows_ports() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    for index in 1_u16..=256 {
        let port_name = format!("COM{index}");
        let opened = serialport::new(port_name.as_str(), 9_600)
            .timeout(Duration::from_millis(35))
            .open();

        match opened {
            Ok(_) => devices.push(windows_device_info(&port_name, "available")),
            Err(err) if windows_port_likely_exists(&err) => {
                devices.push(windows_device_info(&port_name, "busy-or-denied"));
            }
            Err(_) => {}
        }
    }

    Ok(devices)
}

#[cfg(windows)]
fn windows_device_info(port_name: &str, state: &str) -> DeviceInfo {
    let mut metadata = BTreeMap::new();
    metadata.insert("state".to_string(), state.to_string());

    DeviceInfo {
        id: port_name.to_string(),
        kind: PeripheralKind::Serial,
        label: port_name.to_string(),
        path: Some(port_name.to_string()),
        metadata,
    }
}

#[cfg(windows)]
fn windows_port_likely_exists(error: &serialport::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("access is denied")
        || message.contains("permission denied")
        || message.contains("resource busy")
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}
