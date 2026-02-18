use crate::error::{PeriphError, Result};
use crate::hid::{monitor_hid, read_from_hid, scan_hid_devices, write_to_hid};
use crate::peripheral::{
    DeviceInfo, MonitorRequest, PeripheralKind, ReadRequest, ReadResponse, WriteRequest,
    WriteResponse,
};
use crate::serial::{monitor_serial, read_from_serial, scan_serial_ports, write_to_serial};

#[derive(Default)]
pub struct PeripheralService;

impl PeripheralService {
    pub fn scan(&self, kind: Option<PeripheralKind>) -> Result<Vec<DeviceInfo>> {
        match kind {
            None => {
                let mut devices = scan_serial_ports()?;
                if let Ok(mut hid_devices) = scan_hid_devices() {
                    devices.append(&mut hid_devices);
                }
                Ok(devices)
            }
            Some(PeripheralKind::Serial) => scan_serial_ports(),
            Some(PeripheralKind::Hid) => scan_hid_devices(),
            Some(other) => Err(PeriphError::UnsupportedKind(other.to_string())),
        }
    }

    pub fn read(&self, kind: PeripheralKind, request: &ReadRequest) -> Result<ReadResponse> {
        match kind {
            PeripheralKind::Serial => read_from_serial(request),
            PeripheralKind::Hid => read_from_hid(request),
            other => Err(PeriphError::UnsupportedKind(other.to_string())),
        }
    }

    pub fn write(&self, kind: PeripheralKind, request: &WriteRequest) -> Result<WriteResponse> {
        match kind {
            PeripheralKind::Serial => write_to_serial(request),
            PeripheralKind::Hid => write_to_hid(request),
            other => Err(PeriphError::UnsupportedKind(other.to_string())),
        }
    }

    pub fn monitor<F, S>(
        &self,
        kind: PeripheralKind,
        request: &MonitorRequest,
        on_chunk: F,
        should_stop: S,
    ) -> Result<()>
    where
        F: FnMut(ReadResponse),
        S: Fn() -> bool,
    {
        match kind {
            PeripheralKind::Serial => monitor_serial(request, on_chunk, should_stop),
            PeripheralKind::Hid => monitor_hid(request, on_chunk, should_stop),
            other => Err(PeriphError::UnsupportedKind(other.to_string())),
        }
    }
}
