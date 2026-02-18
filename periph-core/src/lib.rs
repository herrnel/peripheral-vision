mod error;
mod hid;
mod peripheral;
mod platform;
mod serial;
mod service;

pub use error::{PeriphError, Result};
pub use peripheral::{
    DeviceInfo, MonitorRequest, PeripheralKind, ReadRequest, ReadResponse, WriteRequest,
    WriteResponse,
};
pub use platform::{
    detect_platform, run_preflight_checks, CheckStatus, PlatformInfo, PreflightCheck,
};
pub use service::PeripheralService;
