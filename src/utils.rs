use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::io::{self, Read};
use std::process;
use std::time::{Duration, Instant};

pub fn parse_hex_string(hex_str: &str) -> Result<u16, String> {
    let cleaned = hex_str.trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(cleaned, 16).map_err(|_| format!("Invalid hex value: {}", hex_str))
}

pub fn list_available_devices(api: &HidApi) {
    println!("\nAvailable HID devices:");
    for device_info in api.device_list() {
        println!(
            "  Vendor: 0x{:04x}, Product: 0x{:04x}, Usage Page: 0x{:04x}, Usage: 0x{:04x}",
            device_info.vendor_id(),
            device_info.product_id(),
            device_info.usage_page(),
            device_info.usage()
        );
        if let Some(product_string) = device_info.product_string() {
            println!("    Product: {}", product_string);
        }
        if let Some(manufacturer_string) = device_info.manufacturer_string() {
            println!("    Manufacturer: {}", manufacturer_string);
        }
    }
}

pub fn find_target_device<'a>(
    api: &'a HidApi,
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    usage_id: u16,
) -> Option<&'a DeviceInfo> {
    api.device_list().find(|device_info| {
        device_info.vendor_id() == vendor_id
            && device_info.product_id() == product_id
            && device_info.usage_page() == usage_page
            && device_info.usage() == usage_id
    })
}

pub fn open_device_or_exit(
    api: &HidApi,
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    usage_id: u16,
) -> HidDevice {
    let device_info = find_target_device(api, vendor_id, product_id, usage_page, usage_id);

    let Some(device_info) = device_info else {
        eprintln!(
            "No HID device found with vendor: 0x{:04x}, product: 0x{:04x}, usage page: 0x{:04x}, usage: 0x{:04x}",
            vendor_id, product_id, usage_page, usage_id
        );
        list_available_devices(api);
        process::exit(1);
    };

    if let Ok(device) = api.open_path(device_info.path()) {
        return device;
    }
    match api.open(vendor_id, product_id) {
        Ok(device) => device,
        Err(e) => {
            eprintln!(
                "Failed to open HID device (vendor: 0x{:04x}, product: 0x{:04x}): {}",
                vendor_id, product_id, e
            );
            list_available_devices(api);
            process::exit(1);
        }
    }
}

pub fn read_stdin_or_exit() -> Vec<u8> {
    let mut input_data = Vec::new();

    if let Err(e) = io::stdin().read_to_end(&mut input_data) {
        eprintln!("Failed to read from stdin: {}", e);
        process::exit(1);
    }

    if input_data.len() > 31 {
        eprintln!(
            "Input data too long: {} bytes (max: 31 bytes)",
            input_data.len()
        );
        process::exit(1);
    }

    input_data
}

pub fn process_response(response_buffer: &[u8], bytes_read: usize, sent_payload: &[u8]) -> bool {
    if bytes_read == 0 {
        println!("No response received, retrying...");
        return false;
    }

    println!("Received {} bytes from device:", bytes_read);

    for i in 0..bytes_read {
        print!("{:02x} ", response_buffer[i]);
    }
    println!();

    let response_data = &response_buffer[..bytes_read];

    // 0x01 is used by the firmware to mark the communication as successful
    if response_data[0] != 0x01 {
        println!(
            "✗ Communication failed (first byte: 0x{:02x}), retrying...",
            response_data[0]
        );
        return false;
    }

    let response_payload = &response_data[1..];

    if response_payload != sent_payload {
        println!("✗ Communication successful but response payload doesn't match, retrying...");
        return false;
    }

    println!("✓ Communication successful - response matches sent data!");

    let has_printable = response_payload.iter().any(|&b| b >= 32 && b <= 126);
    if has_printable {
        let response_text = String::from_utf8_lossy(response_payload);
        let clean_text = response_text.trim_end_matches('\0').trim();
        if !clean_text.is_empty() {
            println!("As text: \"{}\"", clean_text);
        }
    }

    true
}

pub fn send_data_with_retry(
    device: &HidDevice,
    buffer: &[u8],
    vendor_id: u16,
    product_id: u16,
) -> bool {
    let start_time = Instant::now();
    let timeout = Duration::from_secs(5);
    let mut attempt = 0;
    let sent_payload = &buffer[1..];

    while start_time.elapsed() < timeout {
        attempt += 1;

        let bytes_written = match device.write(buffer) {
            Ok(bytes) => bytes,
            Err(e) => {
                println!(
                    "Attempt {}: Failed to write data to device: {}, retrying...",
                    attempt, e
                );
                continue;
            }
        };

        println!(
            "Attempt {}: Sent {} bytes to device (vendor: 0x{:04x}, product: 0x{:04x})",
            attempt, bytes_written, vendor_id, product_id
        );

        let mut response_buffer = [0u8; 32];
        let bytes_read = match device.read_timeout(&mut response_buffer, 200) {
            Ok(bytes) => bytes,
            Err(_) => 0,
        };

        if process_response(&response_buffer, bytes_read, sent_payload) {
            return true;
        }

        if start_time.elapsed() < timeout {
            let wait_time = Duration::from_millis(50 * (1 << (attempt - 1).min(4)));
            std::thread::sleep(wait_time);
        }
    }

    eprintln!(
        "Failed to receive matching response after {} attempts in {} seconds",
        attempt,
        timeout.as_secs()
    );
    false
}
