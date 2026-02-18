use crate::error::{PeriphError, Result};
use crate::peripheral::{
    DeviceInfo, MonitorRequest, PeripheralKind, ReadRequest, ReadResponse, WriteRequest,
    WriteResponse,
};
use hidapi::{HidApi, HidDevice};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
struct VidPidSelector {
    vendor_id: u16,
    product_id: u16,
    serial_number: Option<String>,
}

pub fn scan_hid_devices() -> Result<Vec<DeviceInfo>> {
    let api = HidApi::new()?;
    let mut devices = Vec::new();

    for device in api.device_list() {
        let path = device.path().to_string_lossy().into_owned();
        let vendor_id = device.vendor_id();
        let product_id = device.product_id();
        let selector = build_selector(vendor_id, product_id, device.serial_number());
        let label = device
            .product_string()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HID {:04x}:{:04x}", vendor_id, product_id));
        let mut metadata = BTreeMap::new();

        metadata.insert("vendor_id".to_string(), format!("{vendor_id:04x}"));
        metadata.insert("product_id".to_string(), format!("{product_id:04x}"));
        metadata.insert("selector".to_string(), selector);

        if let Some(value) = device.serial_number() {
            metadata.insert("serial_number".to_string(), value.to_string());
        }
        if let Some(value) = device.manufacturer_string() {
            metadata.insert("manufacturer".to_string(), value.to_string());
        }
        if let Some(value) = device.product_string() {
            metadata.insert("product".to_string(), value.to_string());
        }

        devices.push(DeviceInfo {
            id: path.clone(),
            kind: PeripheralKind::Hid,
            label,
            path: Some(path),
            metadata,
        });
    }

    Ok(devices)
}

pub fn read_from_hid(request: &ReadRequest) -> Result<ReadResponse> {
    let api = HidApi::new()?;
    let device = open_hid_device(&api, &request.port)?;
    let mut buffer = vec![0_u8; request.max_bytes.max(1)];
    let bytes_read = device.read_timeout(buffer.as_mut_slice(), request.timeout_ms as i32)?;

    buffer.truncate(bytes_read);
    Ok(ReadResponse {
        bytes_read,
        data_hex: hex_encode(&buffer),
        data_utf8: String::from_utf8(buffer).ok(),
    })
}

pub fn write_to_hid(request: &WriteRequest) -> Result<WriteResponse> {
    let api = HidApi::new()?;
    let device = open_hid_device(&api, &request.port)?;
    let bytes_written = device.write(&request.data)?;
    Ok(WriteResponse { bytes_written })
}

pub fn monitor_hid<F, S>(request: &MonitorRequest, mut on_chunk: F, should_stop: S) -> Result<()>
where
    F: FnMut(ReadResponse),
    S: Fn() -> bool,
{
    let api = HidApi::new()?;
    let device = open_hid_device(&api, &request.port)?;
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

        let bytes_read = device.read_timeout(buffer.as_mut_slice(), request.timeout_ms as i32)?;
        if bytes_read == 0 {
            continue;
        }

        let chunk = &buffer[..bytes_read];
        on_chunk(ReadResponse {
            bytes_read,
            data_hex: hex_encode(chunk),
            data_utf8: String::from_utf8(chunk.to_vec()).ok(),
        });
    }

    Ok(())
}

fn open_hid_device(api: &HidApi, selector: &str) -> Result<HidDevice> {
    if let Some(parsed) = parse_vid_pid_selector(selector) {
        return open_hid_by_vid_pid(api, &parsed);
    }

    for device in api.device_list() {
        let path = device.path().to_string_lossy();
        if path.as_ref() == selector {
            return device.open_device(api).map_err(Into::into);
        }
    }

    Err(PeriphError::InvalidInput(format!(
        "hid device not found: {selector}. Use `scan --kind hid` to list selectors."
    )))
}

fn open_hid_by_vid_pid(api: &HidApi, selector: &VidPidSelector) -> Result<HidDevice> {
    for device in api.device_list() {
        if device.vendor_id() != selector.vendor_id || device.product_id() != selector.product_id {
            continue;
        }

        if let Some(expected_serial) = selector.serial_number.as_deref() {
            if device.serial_number() != Some(expected_serial) {
                continue;
            }
        }

        return device.open_device(api).map_err(Into::into);
    }

    Err(PeriphError::InvalidInput(format!(
        "hid device not found for selector {:04x}:{:04x}",
        selector.vendor_id, selector.product_id
    )))
}

fn parse_vid_pid_selector(value: &str) -> Option<VidPidSelector> {
    let mut parts = value.splitn(3, ':');
    let vendor_raw = parts.next()?;
    let product_raw = parts.next()?;
    let serial_raw = parts.next();

    if vendor_raw.len() != 4 || product_raw.len() != 4 {
        return None;
    }

    let vendor_id = u16::from_str_radix(vendor_raw, 16).ok()?;
    let product_id = u16::from_str_radix(product_raw, 16).ok()?;
    let serial_number = serial_raw.and_then(|value| {
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    });

    Some(VidPidSelector {
        vendor_id,
        product_id,
        serial_number,
    })
}

fn build_selector(vendor_id: u16, product_id: u16, serial_number: Option<&str>) -> String {
    match serial_number {
        Some(serial) if !serial.is_empty() => format!("{vendor_id:04x}:{product_id:04x}:{serial}"),
        _ => format!("{vendor_id:04x}:{product_id:04x}"),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{build_selector, parse_vid_pid_selector, VidPidSelector};

    #[test]
    fn parse_vid_pid_without_serial() {
        let selector = parse_vid_pid_selector("2341:0043").expect("selector should parse");
        assert_eq!(
            selector,
            VidPidSelector {
                vendor_id: 0x2341,
                product_id: 0x0043,
                serial_number: None,
            }
        );
    }

    #[test]
    fn parse_vid_pid_with_serial() {
        let selector = parse_vid_pid_selector("2341:0043:ABC123").expect("selector should parse");
        assert_eq!(
            selector,
            VidPidSelector {
                vendor_id: 0x2341,
                product_id: 0x0043,
                serial_number: Some("ABC123".to_string()),
            }
        );
    }

    #[test]
    fn parse_vid_pid_rejects_invalid() {
        assert!(parse_vid_pid_selector("COM3").is_none());
        assert!(parse_vid_pid_selector("abcd").is_none());
        assert!(parse_vid_pid_selector("zzzz:1234").is_none());
        assert!(parse_vid_pid_selector("123:1234").is_none());
    }

    #[test]
    fn build_selector_includes_serial_when_present() {
        assert_eq!(
            build_selector(0x2341, 0x0043, Some("ABC123")),
            "2341:0043:ABC123"
        );
        assert_eq!(build_selector(0x2341, 0x0043, None), "2341:0043");
    }
}
