use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PeripheralKind {
    Serial,
    Usb,
    Hid,
    Ble,
    Gpio,
    Camera,
    Audio,
}

impl std::fmt::Display for PeripheralKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Serial => "serial",
            Self::Usb => "usb",
            Self::Hid => "hid",
            Self::Ble => "ble",
            Self::Gpio => "gpio",
            Self::Camera => "camera",
            Self::Audio => "audio",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub kind: PeripheralKind,
    pub label: String,
    pub path: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ReadRequest {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadResponse {
    pub bytes_read: usize,
    pub data_hex: String,
    pub data_utf8: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteResponse {
    pub bytes_written: usize,
}

#[derive(Debug, Clone)]
pub struct MonitorRequest {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub chunk_size: usize,
    pub duration_seconds: Option<u64>,
}
