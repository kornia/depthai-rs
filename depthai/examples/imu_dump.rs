//! Stream raw accelerometer + gyroscope packets for a few seconds.
//!
//! `cargo run --example imu_dump [-- <device-id-or-ip>]`

use std::time::{Duration, Instant};

use depthai::node::Imu;
use depthai::{Device, ImuSensor, Message, Pipeline};

fn main() -> depthai::Result<()> {
    let id = std::env::args().nth(1);
    let dev = Device::open(id.as_deref(), None)?;
    let imu_name = dev.connected_imu()?;
    println!("connected IMU: {imu_name:?}");
    if imu_name.is_empty() || imu_name == "NONE" {
        eprintln!("this board has no IMU");
        return Ok(());
    }

    let pipeline = Pipeline::new(&dev)?;
    let imu = pipeline.create::<Imu>()?;
    imu.enable_sensor(ImuSensor::AccelerometerRaw, 200)?;
    imu.enable_sensor(ImuSensor::GyroscopeRaw, 200)?;
    imu.set_batch_report_threshold(5)?;
    imu.set_max_batch_reports(5)?;
    let q = imu.out()?.create_output_queue(50, false)?;
    pipeline.start()?;

    let start = Instant::now();
    let (mut batches, mut samples, mut holes) = (0u32, 0u32, 0u32);
    let mut packets = Vec::new();
    while start.elapsed() < Duration::from_secs(3) {
        let Some(batch) = q.get(Duration::from_millis(500))? else {
            continue;
        };
        batches += 1;
        packets.clear();
        batch.packets_into(&mut packets)?;
        for p in &packets {
            if p.accelerometer.timestamp.is_zero() || p.gyroscope.timestamp.is_zero() {
                holes += 1;
                continue;
            }
            samples += 1;
            if samples % 200 == 1 {
                println!(
                    "acc={:?} gyro={:?} ts={:?} batch_ts={:?}",
                    p.accelerometer.xyz(),
                    p.gyroscope.xyz(),
                    p.accelerometer.timestamp.as_duration(),
                    batch.timestamp(),
                );
            }
        }
    }
    println!(
        "{batches} batches, {samples} samples ({:.0} Hz), {holes} zero-timestamp holes",
        samples as f32 / 3.0
    );
    pipeline.stop()?;
    Ok(())
}
