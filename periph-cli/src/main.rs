use clap::{Args, Parser, Subcommand, ValueEnum};
use periph_core::{
    detect_platform, run_preflight_checks, CheckStatus, MonitorRequest, PeripheralKind,
    PeripheralService, ReadRequest, ReadResponse, WriteRequest,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Parser)]
#[command(name = "periph")]
#[command(about = "Cross-platform peripheral CLI for Unix, Linux, Windows, and WSL")]
struct Cli {
    #[arg(long, global = true, help = "Output JSON")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Platform,
    Preflight,
    Scan(ScanArgs),
    Read(ReadArgs),
    Write(WriteArgs),
    Monitor(MonitorArgs),
}

#[derive(Debug, Args)]
struct ScanArgs {
    #[arg(long, value_enum, default_value = "all")]
    kind: ScanKindArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScanKindArg {
    All,
    Serial,
    Usb,
    Hid,
    Ble,
    Gpio,
    Camera,
    Audio,
}

impl ScanKindArg {
    fn as_kind(self) -> Option<PeripheralKind> {
        match self {
            Self::All => None,
            Self::Serial => Some(PeripheralKind::Serial),
            Self::Usb => Some(PeripheralKind::Usb),
            Self::Hid => Some(PeripheralKind::Hid),
            Self::Ble => Some(PeripheralKind::Ble),
            Self::Gpio => Some(PeripheralKind::Gpio),
            Self::Camera => Some(PeripheralKind::Camera),
            Self::Audio => Some(PeripheralKind::Audio),
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum KindArg {
    Serial,
    Usb,
    Hid,
    Ble,
    Gpio,
    Camera,
    Audio,
}

impl KindArg {
    fn as_kind(self) -> PeripheralKind {
        match self {
            Self::Serial => PeripheralKind::Serial,
            Self::Usb => PeripheralKind::Usb,
            Self::Hid => PeripheralKind::Hid,
            Self::Ble => PeripheralKind::Ble,
            Self::Gpio => PeripheralKind::Gpio,
            Self::Camera => PeripheralKind::Camera,
            Self::Audio => PeripheralKind::Audio,
        }
    }
}

#[derive(Debug, Args)]
struct ReadArgs {
    #[arg(long, value_enum, default_value = "serial")]
    kind: KindArg,
    #[arg(
        long,
        visible_alias = "device",
        help = "Serial port or HID selector/path (for HID: vid:pid[:serial])"
    )]
    port: String,
    #[arg(long, default_value_t = 115_200)]
    baud: u32,
    #[arg(long, default_value_t = 1500)]
    timeout_ms: u64,
    #[arg(long, default_value_t = 1024)]
    bytes: usize,
    #[arg(long, value_enum, default_value = "both")]
    output: OutputMode,
}

#[derive(Debug, Args)]
struct WriteArgs {
    #[arg(long, value_enum, default_value = "serial")]
    kind: KindArg,
    #[arg(
        long,
        visible_alias = "device",
        help = "Serial port or HID selector/path (for HID: vid:pid[:serial])"
    )]
    port: String,
    #[arg(long, default_value_t = 115_200)]
    baud: u32,
    #[arg(long, default_value_t = 1500)]
    timeout_ms: u64,
    #[arg(long, help = "Payload string or hex bytes depending on --hex")]
    data: String,
    #[arg(long, help = "Interpret --data as hex (spaces, :, -, _ allowed)")]
    hex: bool,
}

#[derive(Debug, Args)]
struct MonitorArgs {
    #[arg(long, value_enum, default_value = "serial")]
    kind: KindArg,
    #[arg(
        long,
        visible_alias = "device",
        help = "Serial port or HID selector/path (for HID: vid:pid[:serial])"
    )]
    port: String,
    #[arg(long, default_value_t = 115_200)]
    baud: u32,
    #[arg(long, default_value_t = 300)]
    timeout_ms: u64,
    #[arg(long, default_value_t = 256)]
    chunk_size: usize,
    #[arg(long, value_enum, default_value = "both")]
    output: OutputMode,
    #[arg(long)]
    duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputMode {
    Utf8,
    Hex,
    Both,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> periph_core::Result<()> {
    let cli = Cli::parse();
    let service = PeripheralService;

    match cli.command {
        Command::Platform => {
            let platform = detect_platform();
            if cli.json {
                print_json(&platform)?;
            } else {
                println!("OS: {}", platform.os);
                println!("Family: {}", platform.os_family);
                println!("Arch: {}", platform.arch);
                println!("WSL: {}", platform.is_wsl);
            }
        }
        Command::Preflight => {
            let checks = run_preflight_checks();
            if cli.json {
                print_json(&checks)?;
            } else {
                for check in checks {
                    let status = match check.status {
                        CheckStatus::Pass => "PASS",
                        CheckStatus::Warn => "WARN",
                        CheckStatus::Fail => "FAIL",
                    };
                    println!("[{status}] {}: {}", check.name, check.details);
                }
            }
        }
        Command::Scan(args) => {
            let devices = service.scan(args.kind.as_kind())?;
            if cli.json {
                print_json(&devices)?;
            } else if devices.is_empty() {
                println!("No devices found.");
            } else {
                for device in devices {
                    println!(
                        "{} [{}] {}",
                        device.label,
                        device.kind,
                        device.path.unwrap_or_else(|| "-".to_string())
                    );
                }
            }
        }
        Command::Read(args) => {
            let response = service.read(
                args.kind.as_kind(),
                &ReadRequest {
                    port: args.port,
                    baud_rate: args.baud,
                    timeout_ms: args.timeout_ms,
                    max_bytes: args.bytes,
                },
            )?;

            if cli.json {
                print_json(&response)?;
            } else {
                print_read_response(&response, args.output);
            }
        }
        Command::Write(args) => {
            let payload = if args.hex {
                parse_hex(&args.data)?
            } else {
                args.data.into_bytes()
            };
            let response = service.write(
                args.kind.as_kind(),
                &WriteRequest {
                    port: args.port,
                    baud_rate: args.baud,
                    timeout_ms: args.timeout_ms,
                    data: payload,
                },
            )?;

            if cli.json {
                print_json(&response)?;
            } else {
                println!("Wrote {} byte(s).", response.bytes_written);
            }
        }
        Command::Monitor(args) => {
            let stop_signal = Arc::new(AtomicBool::new(false));
            let stop_signal_for_handler = Arc::clone(&stop_signal);
            ctrlc::set_handler(move || {
                stop_signal_for_handler.store(true, Ordering::SeqCst);
            })
            .map_err(|err| periph_core::PeriphError::InvalidInput(err.to_string()))?;

            if !cli.json {
                println!(
                    "Monitoring {} at {} baud. Press Ctrl-C to stop.",
                    args.port, args.baud
                );
            }

            let monitor_request = MonitorRequest {
                port: args.port,
                baud_rate: args.baud,
                timeout_ms: args.timeout_ms,
                chunk_size: args.chunk_size,
                duration_seconds: args.duration_seconds,
            };

            service.monitor(
                args.kind.as_kind(),
                &monitor_request,
                |response| {
                    if cli.json {
                        if let Ok(json) = serde_json::to_string(&response) {
                            println!("{json}");
                        }
                    } else {
                        print_read_response(&response, args.output);
                    }
                },
                || stop_signal.load(Ordering::SeqCst),
            )?;
        }
    }

    Ok(())
}

fn print_read_response(response: &ReadResponse, output: OutputMode) {
    match output {
        OutputMode::Utf8 => println!(
            "{}",
            response
                .data_utf8
                .clone()
                .unwrap_or_else(|| "<non-utf8>".to_string())
        ),
        OutputMode::Hex => println!("{}", response.data_hex),
        OutputMode::Both => {
            println!("bytes: {}", response.bytes_read);
            println!("hex: {}", response.data_hex);
            println!(
                "utf8: {}",
                response
                    .data_utf8
                    .clone()
                    .unwrap_or_else(|| "<non-utf8>".to_string())
            );
        }
    }
}

fn parse_hex(value: &str) -> periph_core::Result<Vec<u8>> {
    let normalized: String = value
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != ':' && *ch != '-' && *ch != '_')
        .collect();

    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    if normalized.len() % 2 != 0 {
        return Err(periph_core::PeriphError::InvalidInput(
            "hex payload must have an even number of digits".to_string(),
        ));
    }

    let mut bytes = Vec::with_capacity(normalized.len() / 2);
    let chars = normalized.as_bytes();
    for pair in chars.chunks(2) {
        let pair_str = std::str::from_utf8(pair)
            .map_err(|err| periph_core::PeriphError::InvalidInput(err.to_string()))?;
        let byte = u8::from_str_radix(pair_str, 16).map_err(|_| {
            periph_core::PeriphError::InvalidInput(format!("invalid hex byte: {pair_str}"))
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn print_json<T: serde::Serialize>(value: &T) -> periph_core::Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|err| periph_core::PeriphError::InvalidInput(err.to_string()))?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_hex;

    #[test]
    fn parse_hex_accepts_separators() {
        let value = parse_hex("48 65:6c-6c_6f").expect("hex should parse");
        assert_eq!(value, b"Hello");
    }

    #[test]
    fn parse_hex_rejects_odd_digits() {
        let err = parse_hex("abc").expect_err("should reject odd digits");
        assert!(err
            .to_string()
            .contains("hex payload must have an even number of digits"));
    }
}
