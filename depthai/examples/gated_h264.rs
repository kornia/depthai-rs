//! On-device H.264 that the host switches on and off at runtime: camera NV12 →
//! `Gate` → `VideoEncoder`. While the gate is closed the encoder gets nothing and
//! no bitstream crosses the link — the pattern for "stream only on detection"
//! (`GateControl::open_for(n, fps)` passes a burst) or for sparing a saturated
//! PoE/USB2 link.
//!
//! `cargo run --example gated_h264 [-- <device-id-or-ip>]`

use std::time::{Duration, Instant};

use depthai::node::{Camera, Gate, VideoEncoder};
use depthai::{
    CameraBoardSocket, Device, GateControl, ImgFrameType, ImgResizeMode, Message, Pipeline,
    VideoEncoderProfile,
};

fn main() -> depthai::Result<()> {
    let id = std::env::args().nth(1);
    let (w, h, fps) = (640u32, 360u32, 30.0f32);
    let dev = Device::open(id.as_deref(), None)?;
    let pipeline = Pipeline::new(&dev)?;

    let color = pipeline.create::<Camera>()?;
    color.build(CameraBoardSocket::CamA)?;
    let nv12 = color.request_output(
        (w, h),
        Some(ImgFrameType::Nv12),
        ImgResizeMode::Crop,
        Some(fps),
        Some(true),
    )?;

    let gate = pipeline.create::<Gate>()?;
    nv12.link(&gate.input()?)?;
    let control = gate.input_control()?.create_input_queue(4, false)?;

    let enc = pipeline.create::<VideoEncoder>()?;
    enc.set_default_profile_preset(fps, VideoEncoderProfile::H264Baseline)?;
    enc.set_keyframe_frequency(8)?;
    enc.set_bitrate_kbps(2000)?;
    gate.output()?
        .cast::<depthai::ImgFrame>()
        .link(&enc.input()?)?;
    let video = enc.bitstream()?.create_output_queue(30, false)?;

    pipeline.start()?;
    println!("gate open; toggling every 3 s, then a 15-frame burst — bytes per second:");

    let start = Instant::now();
    let mut open = true;
    let mut last_toggle = Instant::now();
    let mut window = Instant::now();
    let (mut frames, mut bytes) = (0u32, 0usize);
    while start.elapsed() < Duration::from_secs(14) {
        if let Some(au) = video.get(Duration::from_millis(100))? {
            frames += 1;
            bytes += au.data().len();
            let _ = au.sequence_num();
        }
        if window.elapsed() >= Duration::from_secs(1) {
            println!(
                "  gate {:<6} {frames:>3} AUs {:>6} kB",
                if open { "open" } else { "closed" },
                bytes / 1024
            );
            frames = 0;
            bytes = 0;
            window = Instant::now();
        }
        if last_toggle.elapsed() >= Duration::from_secs(3) {
            open = !open;
            control.send(&if open {
                GateControl::open()?
            } else {
                GateControl::close()?
            })?;
            last_toggle = Instant::now();
        }
    }
    // A burst: 15 frames at 5 fps, then closed again.
    control.send(&GateControl::close()?)?;
    control.send(&GateControl::open_for(15, Some(5))?)?;
    let t = Instant::now();
    let mut burst = 0;
    while t.elapsed() < Duration::from_secs(5) {
        if video.get(Duration::from_millis(100))?.is_some() {
            burst += 1;
        }
    }
    println!("burst delivered {burst} access units (asked for 15)");
    pipeline.stop()?;
    Ok(())
}
