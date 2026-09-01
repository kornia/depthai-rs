//! Enumerate every OAK visible on USB/network and print its state.
//!
//! `cargo run --example list_devices`

fn main() -> depthai::Result<()> {
    println!("depthai-core {}", depthai::build_version());
    let devices = depthai::Device::all_available()?;
    if devices.is_empty() {
        println!("no devices found");
        return Ok(());
    }
    for d in &devices {
        println!(
            "{:<24} id={:<20} state={:?} protocol={} platform={}",
            d.name, d.device_id, d.state, d.protocol, d.platform
        );
    }
    Ok(())
}
