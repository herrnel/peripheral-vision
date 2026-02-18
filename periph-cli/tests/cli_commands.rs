use serde_json::Value;
use std::process::Command;

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_periph-cli"))
        .args(args)
        .output()
        .expect("periph-cli should run")
}

fn stdout_string(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

fn stderr_string(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf8")
}

#[test]
fn platform_json_contains_expected_fields() {
    let output = run_cli(&["--json", "platform"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        stderr_string(&output)
    );

    let value: Value = serde_json::from_str(&stdout_string(&output)).expect("json output");
    assert!(value["os"].is_string());
    assert!(value["os_family"].is_string());
    assert!(value["arch"].is_string());
    assert!(value["is_wsl"].is_boolean());
}

#[test]
fn preflight_json_returns_array() {
    let output = run_cli(&["--json", "preflight"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        stderr_string(&output)
    );

    let value: Value = serde_json::from_str(&stdout_string(&output)).expect("json output");
    assert!(value.is_array());
}

#[test]
fn scan_serial_json_returns_array() {
    let output = run_cli(&["--json", "scan", "--kind", "serial"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        stderr_string(&output)
    );

    let value: Value = serde_json::from_str(&stdout_string(&output)).expect("json output");
    assert!(value.is_array());
}

#[test]
fn scan_usb_reports_unsupported_kind() {
    let output = run_cli(&["scan", "--kind", "usb"]);
    assert!(
        !output.status.success(),
        "stdout: {}",
        stdout_string(&output)
    );
    assert!(
        stderr_string(&output).contains("unsupported peripheral kind: usb"),
        "stderr: {}",
        stderr_string(&output)
    );
}

#[test]
fn write_hex_rejects_odd_length_input() {
    let output = run_cli(&[
        "write", "--kind", "serial", "--port", "dummy", "--hex", "--data", "abc",
    ]);
    assert!(
        !output.status.success(),
        "stdout: {}",
        stdout_string(&output)
    );
    assert!(
        stderr_string(&output).contains("hex payload must have an even number of digits"),
        "stderr: {}",
        stderr_string(&output)
    );
}
