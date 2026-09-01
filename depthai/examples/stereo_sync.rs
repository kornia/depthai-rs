//! A time-synced GRAY8 stereo pair from CAM_B/CAM_C through a Sync node, plus
//! the factory stereo calibration. The de-policied core of a stereo driver.
//!
//! `cargo run --example stereo_sync [-- <device-id-or-ip>]`

use std::time::{Duration, Instant};

use depthai::node::{Camera, Sync};
use depthai::{
    CameraBoardSocket, Device, ImgFrame, ImgFrameType, ImgResizeMode, LengthUnit, Message,
    MessageGroup, Pipeline,
};

fn main() -> depthai::Result<()> {
    let id = std::env::args().nth(1);
    let (w, h, fps) = (640u32, 400u32, 30.0f32);

    let dev = Device::open(id.as_deref(), None)?;
    println!(
        "device {} ({}) on {:?}",
        dev.name()?,
        dev.id()?,
        dev.usb_speed()?
    );
    let cams = dev.connected_cameras()?;
    println!("cameras: {cams:?}");
    if !cams.contains(&CameraBoardSocket::CamB) || !cams.contains(&CameraBoardSocket::CamC) {
        eprintln!("no stereo pair (CAM_B/CAM_C) on this device");
        return Ok(());
    }

    // Calibration: intrinsics at the streamed size, the calibrated (not spec)
    // left->right extrinsic in metres.
    let calib = dev.read_calibration()?;
    let k = calib.camera_intrinsics(CameraBoardSocket::CamB, Some((w, h)))?;
    let t = calib.camera_extrinsics(
        CameraBoardSocket::CamB,
        CameraBoardSocket::CamC,
        false,
        LengthUnit::Meter,
    )?;
    let baseline = (t[0][3].powi(2) + t[1][3].powi(2) + t[2][3].powi(2)).sqrt();
    println!(
        "left fx={:.1} fy={:.1} cx={:.1} cy={:.1}  baseline={baseline:.4} m",
        k[0][0], k[1][1], k[0][2], k[1][2]
    );

    let pipeline = Pipeline::new(&dev)?;
    let left = pipeline.create::<Camera>()?;
    left.build(CameraBoardSocket::CamB)?;
    let right = pipeline.create::<Camera>()?;
    right.build(CameraBoardSocket::CamC)?;
    // Raw (undistort = false): a host rectifier wants untouched pixels.
    let lo = left.request_output(
        (w, h),
        Some(ImgFrameType::Gray8),
        ImgResizeMode::Crop,
        Some(fps),
        Some(false),
    )?;
    let ro = right.request_output(
        (w, h),
        Some(ImgFrameType::Gray8),
        ImgResizeMode::Crop,
        Some(fps),
        Some(false),
    )?;

    let sync = pipeline.create::<Sync>()?;
    sync.set_sync_threshold(Duration::from_nanos((1_000_000_000.0 / fps / 2.0) as u64))?;
    lo.link(&sync.input("left")?)?;
    ro.link(&sync.input("right")?)?;
    let q = sync.out()?.create_output_queue(4, false)?;

    pipeline.start()?;
    println!("streaming {w}x{h}@{fps} for 5 s ...");

    let start = Instant::now();
    let mut n = 0u32;
    while start.elapsed() < Duration::from_secs(5) {
        let Some(group): Option<MessageGroup> = q.get(Duration::from_secs(1))? else {
            eprintln!("timeout waiting for a pair");
            continue;
        };
        let l: ImgFrame = group.get("left")?.expect("left eye");
        let r: ImgFrame = group.get("right")?.expect("right eye");
        n += 1;
        if n % 30 == 1 {
            println!(
                "#{:<4} {}x{} {:?} stride={} seq={} ts={:?} synced={} interval={:?}",
                n,
                l.width(),
                l.height(),
                l.img_type(),
                l.stride(),
                l.sequence_num(),
                l.timestamp(),
                group.is_synced(Duration::from_millis(16))?,
                group.interval()?,
            );
            assert_eq!((l.width(), l.height()), (r.width(), r.height()));
        }
    }
    println!("{n} pairs in 5 s ({:.1} fps)", n as f32 / 5.0);
    pipeline.stop()?;
    Ok(())
}
