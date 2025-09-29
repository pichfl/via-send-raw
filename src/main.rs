use clap::Parser;
use hidapi::HidApi;
use std::process;

mod utils;
use utils::{open_device_or_exit, parse_hex_string, read_stdin_or_exit, send_data_with_retry};

#[derive(Parser)]
#[command(name = "viasendraw")]
#[command(about = "A tiny util to send raw data to VIA-compatible keyboards")]
#[command(version)]
struct Args {
    /// Vendor ID (hex format, e.g., 0xcb00)
    #[arg(short = 'v', long = "vendor")]
    vendor_id: String,

    /// Product ID (hex format, e.g., 0x2006)
    #[arg(short = 'p', long = "product")]
    product_id: String,

    /// HID Usage Page (default: 0xFF60 for QMK compatibility)
    #[arg(long = "usage-page", default_value = "0xFF60")]
    usage_page: String,

    /// HID Usage ID (default: 0x61 for QMK compatibility)
    #[arg(long = "usage-id", default_value = "0x61")]
    usage_id: String,
}

fn parse_args_or_exit(args: &Args) -> (u16, u16, u16, u16) {
    let vendor_id = parse_hex_string(&args.vendor_id).unwrap_or_else(|e| {
        eprintln!("Error parsing vendor ID: {}", e);
        process::exit(1);
    });

    let product_id = parse_hex_string(&args.product_id).unwrap_or_else(|e| {
        eprintln!("Error parsing product ID: {}", e);
        process::exit(1);
    });

    let usage_page = parse_hex_string(&args.usage_page).unwrap_or_else(|e| {
        eprintln!("Error parsing usage page: {}", e);
        process::exit(1);
    });

    let usage_id = parse_hex_string(&args.usage_id).unwrap_or_else(|e| {
        eprintln!("Error parsing usage ID: {}", e);
        process::exit(1);
    });

    (vendor_id, product_id, usage_page, usage_id)
}

fn main() {
    let args = Args::parse();
    let (vendor_id, product_id, usage_page, usage_id) = parse_args_or_exit(&args);

    let api = HidApi::new().unwrap_or_else(|e| {
        eprintln!("Failed to initialize HID API: {}", e);
        process::exit(1);
    });

    let device = open_device_or_exit(&api, vendor_id, product_id, usage_page, usage_id);

    let input_data = read_stdin_or_exit();

    let mut buffer = [0u8; 32];
    buffer[0] = 0xff; // this is the marker for VIA custom commands
    buffer[1..1 + input_data.len()].copy_from_slice(&input_data);

    if !send_data_with_retry(&device, &buffer, vendor_id, product_id) {
        process::exit(1);
    }
}
