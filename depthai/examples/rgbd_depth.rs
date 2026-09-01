//! Colour (RGB888i) + StereoDepth aligned to it + on-device H.264, each on its
//! own queue. Prints per-stream counts for 5 s.
//!
//! `cargo run --example rgbd_depth [-- <device-id-or-ip>]`

use std::time::{Duration, Instant};

use depthai::node::{Camera, StereoDepth, VideoEncoder};
use depthai::{
    CameraBoardSocket, Device, ImgFrameType, ImgResizeMode, Pipeline, StereoPresetMode,
    VideoEncoderProfile,
};

fn main() -> depthai::Result<()> {
    let id = std::env::args().nth(1);
    let (w, h, fps) = (640u32, 360u32, 30.0f32);
    let dev = Device::open(id.as_deref(), None)?;
    let cams = dev.connected_cameras()?;
    let has_stereo =
        cams.contains(&CameraBoardSocket::CamB) && cams.contains(&CameraBoardSocket::CamC);
    println!("cameras {cams:?} stereo={has_stereo}");

    let pipeline = Pipeline::new(&dev)?;
    let color = pipeline.create::<Camera>()?;
    color.build(CameraBoardSocket::CamA)?;
    let rgb_out = color.request_output(
        (w, h),
        Some(ImgFrameType::Rgb888i),
        ImgResizeMode::Crop,
        Some(10.0),
        Some(true),
    )?;
    let rgb_q = rgb_out.create_output_queue(4, false)?;

    let depth_q = if has_stereo {
        let left = pipeline.create::<Camera>()?;
        left.build(CameraBoardSocket::CamB)?;
        let right = pipeline.create::<Camera>()?;
        right.build(CameraBoardSocket::CamC)?;
        let stereo = pipeline.create::<StereoDepth>()?;
        stereo.set_default_profile_preset(StereoPresetMode::Robotics)?;
        stereo.set_left_right_check(true)?;
        stereo.set_subpixel(true)?;
        stereo
            .post_processing()
            .set_spatial_filter_enable(true)?
            .set_temporal_filter_enable(true)?
            .set_threshold_filter(400, 8000)?;
        left.request_output((640, 400), None, ImgResizeMode::Crop, Some(fps), None)?
            .link(&stereo.left()?)?;
        right
            .request_output((640, 400), None, ImgResizeMode::Crop, Some(fps), None)?
            .link(&stereo.right()?)?;
        rgb_out.link(&stereo.input_align_to()?)?;
        // setOutputSize is not supported on RVC4.
        if dev.platform()? != depthai::Platform::Rvc4 {
            stereo.set_output_size(w / 2, h / 2)?;
        }
        Some(stereo.depth()?.create_output_queue(4, false)?)
    } else {
        None
    };

    let nv12 = color.request_output(
        (w, h),
        Some(ImgFrameType::Nv12),
        ImgResizeMode::Crop,
        Some(fps),
        Some(true),
    )?;
    let enc = pipeline.create::<VideoEncoder>()?;
    enc.set_default_profile_preset(fps, VideoEncoderProfile::H264Baseline)?;
    enc.set_keyframe_frequency(8)?;
    enc.set_bitrate_kbps(2000)?;
    nv12.link(&enc.input()?)?;
    let video_q = enc.bitstream()?.create_output_queue(30, false)?;

    pipeline.start()?;
    let _ = dev.set_ir_laser_dot_projector_intensity(0.8, None);

    let start = Instant::now();
    let (mut rgb, mut depth, mut video, mut video_bytes) = (0u32, 0u32, 0u32, 0usize);
    let mut depth_dims = (0, 0);
    while start.elapsed() < Duration::from_secs(5) {
        // Block on the colour stream (10 fps) and drain the others opportunistically:
        // no busy-polling three queues.
        if let Some(f) = rgb_q.get(Duration::from_millis(200))? {
            rgb += 1;
            debug_assert_eq!(f.data().len(), (f.width() * f.height() * 3) as usize);
        }
        if let Some(q) = &depth_q {
            while let Some(d) = q.try_get()? {
                depth += 1;
                depth_dims = (d.width(), d.height());
                if depth == 1 {
                    println!(
                        "depth {:?} stride={} len={}",
                        d.img_type(),
                        d.stride(),
                        d.data().len()
                    );
                }
            }
        }
        while let Some(v) = video_q.try_get()? {
            video += 1;
            video_bytes += v.data().len();
        }
    }
    println!(
        "5 s: rgb={rgb} depth={depth} ({}x{}) video={video} ({} kB)",
        depth_dims.0,
        depth_dims.1,
        video_bytes / 1024
    );
    pipeline.stop()?;
    Ok(())
}
