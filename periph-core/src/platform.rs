use crate::hid::scan_hid_devices;
use crate::peripheral::DeviceInfo;
use crate::serial::scan_serial_ports;
use serde::Serialize;
#[cfg(target_os = "linux")]
use std::fs;

#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub os: String,
    pub os_family: String,
    pub arch: String,
    pub is_wsl: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreflightCheck {
    pub name: String,
    pub status: CheckStatus,
    pub details: String,
}

pub fn detect_platform() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        os_family: std::env::consts::FAMILY.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        is_wsl: detect_wsl(),
    }
}

pub fn run_preflight_checks() -> Vec<PreflightCheck> {
    let platform = detect_platform();
    let mut checks = Vec::new();

    checks.push(PreflightCheck {
        name: "platform".to_string(),
        status: CheckStatus::Pass,
        details: format!(
            "{} / {} / {} (wsl: {})",
            platform.os, platform.os_family, platform.arch, platform.is_wsl
        ),
    });

    if platform.is_wsl {
        checks.push(PreflightCheck {
            name: "wsl-usb-note".to_string(),
            status: CheckStatus::Warn,
            details: "WSL needs USB passthrough (usbipd-win) for direct USB/serial devices."
                .to_string(),
        });
    }

    checks.push(serial_scan_check());
    checks.push(hid_scan_check());
    checks
}

fn serial_scan_check() -> PreflightCheck {
    match scan_serial_ports() {
        Ok(devices) if devices.is_empty() => PreflightCheck {
            name: "serial-scan".to_string(),
            status: CheckStatus::Warn,
            details:
                "No serial ports found. Device may be disconnected or permissions may be missing."
                    .to_string(),
        },
        Ok(devices) => serial_found_check(devices),
        Err(err) => PreflightCheck {
            name: "serial-scan".to_string(),
            status: CheckStatus::Fail,
            details: err.to_string(),
        },
    }
}

fn serial_found_check(devices: Vec<DeviceInfo>) -> PreflightCheck {
    PreflightCheck {
        name: "serial-scan".to_string(),
        status: CheckStatus::Pass,
        details: format!("Found {} serial port(s).", devices.len()),
    }
}

fn hid_scan_check() -> PreflightCheck {
    match scan_hid_devices() {
        Ok(devices) if devices.is_empty() => PreflightCheck {
            name: "hid-scan".to_string(),
            status: CheckStatus::Warn,
            details: "No HID devices found.".to_string(),
        },
        Ok(devices) => PreflightCheck {
            name: "hid-scan".to_string(),
            status: CheckStatus::Pass,
            details: format!("Found {} HID device(s).", devices.len()),
        },
        Err(err) => PreflightCheck {
            name: "hid-scan".to_string(),
            status: CheckStatus::Warn,
            details: format!("HID scan unavailable: {err}"),
        },
    }
}

fn detect_wsl() -> bool {
    #[cfg(not(target_os = "linux"))]
    {
        false
    }

    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WSL_DISTRO_NAME").is_some()
            || std::env::var_os("WSL_INTEROP").is_some()
        {
            return true;
        }

        if let Ok(content) = fs::read_to_string("/proc/sys/kernel/osrelease") {
            let lower = content.to_ascii_lowercase();
            return lower.contains("microsoft") || lower.contains("wsl");
        }

        false
    }
}
