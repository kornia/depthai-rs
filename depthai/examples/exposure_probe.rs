//! Per-frame exposure on every stream a stereo + depth + H.264 driver reads: the raw left mono
//! (`StereoDepth::synced_left`), the rectified pair, the depth map aligned to CAM_B, and the
//! colour H.264 bitstream. Prints each stream's first frames and then, per stream, how many
//! frames carried an exposure and its range, and each stream's stamp offset from the rectified
//! left eye at END and at MIDDLE of exposure (`timestamp - exposure / 2`).
//!
//! `cargo run --release --example exposure_probe [-- [--usb-super] <device-id-or-ip> [seconds] [fps]]`
//!
//! The link is capped at USB2 unless `--usb-super`, as xle-oakd's `open_device` does: depthai's
//! default boots the device into a USB3 descriptor that a USB2 link cannot reconnect to, and the
//! open fails with `X_LINK_DEVICE_NOT_FOUND`. Every visible device and its state is listed first.
//!
//! It answers: does `StereoDepth` carry its inputs' exposure onto the rectified eyes and the
//! depth map, and does the encoder carry the colour sensor's onto the bitstream? A stream
//! reporting 0 everywhere does not.

use std::time::{Duration, Instant};

use depthai::node::{Camera, StereoDepth, VideoEncoder};
use depthai::{
    CameraBoardSocket, Device, ImgFrame, ImgFrameType, ImgResizeMode, Message, OutputQueue,
    Pipeline, StereoPresetMode, UsbSpeed, VideoEncoderProfile,
};

/// One stream's frames as `(end stamp ns, exposure)`.
struct Stream {
    name: &'static str,
    q: OutputQueue<ImgFrame>,
    frames: Vec<(i64, Duration)>,
}

impl Stream {
    fn drain(&mut self) -> depthai::Result<()> {
        while let Some(f) = self.q.try_get()? {
            if self.frames.len() < 3 {
                println!(
                    "{:<14} seq={:<6} end={} ns  exposure={:?}",
                    self.name,
                    f.sequence_num(),
                    f.timestamp_ns(),
                    f.exposure()
                );
            }
            self.frames.push((f.timestamp_ns(), f.exposure()));
        }
        Ok(())
    }
}

fn mid(end: i64, e: Duration) -> i64 {
    end - e.as_nanos() as i64 / 2
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    v.sort_by(f64::total_cmp);
    v.get(v.len() / 2).copied()
}

fn main() -> depthai::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let usb_super = args.iter().any(|a| a == "--usb-super");
    args.retain(|a| a != "--usb-super");
    let mut args = args.into_iter();
    let id = args.next().filter(|s| !s.is_empty());
    let secs: u64 = args.next().map_or(5, |s| s.parse().expect("seconds"));
    let fps: f32 = args.next().map_or(15.0, |s| s.parse().expect("fps"));
    let (w, h) = (640u32, 400u32);

    for d in Device::all_available()? {
        println!(
            "found {} ({}) {:?} {:?} {:?} status={}",
            d.device_id, d.name, d.state, d.protocol, d.platform, d.status
        );
    }
    let speed = if usb_super {
        UsbSpeed::Super
    } else {
        UsbSpeed::High
    };
    let dev = Device::open(id.as_deref(), Some(speed))?;
    println!("opened on {:?} (cap {speed:?})", dev.usb_speed()?);
    let cams = dev.connected_cameras()?;
    println!("device {} cameras {cams:?}  {w}x{h}@{fps}", dev.id()?);

    let pipeline = Pipeline::new(&dev)?;
    let left = pipeline.create::<Camera>()?;
    left.build(CameraBoardSocket::CamB)?;
    let right = pipeline.create::<Camera>()?;
    right.build(CameraBoardSocket::CamC)?;
    let stereo = pipeline.create::<StereoDepth>()?;
    stereo.set_default_profile_preset(StereoPresetMode::Robotics)?;
    stereo.set_depth_align(CameraBoardSocket::CamB)?;
    stereo.set_output_size(w, h)?;
    let gray = |cam: &Camera| {
        cam.request_output(
            (w, h),
            Some(ImgFrameType::Gray8),
            ImgResizeMode::Crop,
            Some(fps),
            Some(false),
        )
    };
    gray(&left)?.link(&stereo.left()?)?;
    gray(&right)?.link(&stereo.right()?)?;

    let mut streams = vec![
        Stream {
            name: "rect_left",
            q: stereo.rectified_left()?.create_output_queue(8, false)?,
            frames: Vec::new(),
        },
        Stream {
            name: "rect_right",
            q: stereo.rectified_right()?.create_output_queue(8, false)?,
            frames: Vec::new(),
        },
        Stream {
            name: "raw_left",
            q: stereo.synced_left()?.create_output_queue(8, false)?,
            frames: Vec::new(),
        },
        Stream {
            name: "depth",
            q: stereo.depth()?.create_output_queue(8, false)?,
            frames: Vec::new(),
        },
    ];
    if cams.contains(&CameraBoardSocket::CamA) {
        let color = pipeline.create::<Camera>()?;
        color.build(CameraBoardSocket::CamA)?;
        let enc = pipeline.create::<VideoEncoder>()?;
        enc.set_default_profile_preset(fps, VideoEncoderProfile::H264Baseline)?;
        color
            .request_output(
                (w, h),
                Some(ImgFrameType::Nv12),
                ImgResizeMode::Crop,
                Some(fps),
                Some(true),
            )?
            .link(&enc.input()?)?;
        streams.push(Stream {
            name: "color_h264",
            q: enc.bitstream()?.create_output_queue(30, false)?,
            frames: Vec::new(),
        });
    }

    pipeline.start()?;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(secs) {
        for s in &mut streams {
            s.drain()?;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    pipeline.stop()?;

    println!("\n{secs} s:");
    let reference = streams[0].frames.clone();
    for s in &streams {
        let with: Vec<f64> = s
            .frames
            .iter()
            .filter(|f| f.1 > Duration::ZERO)
            .map(|f| f.1.as_secs_f64() * 1e3)
            .collect();
        let lo = with.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = with.iter().copied().fold(0.0, f64::max);
        // Offset from the nearest rectified-left frame by END stamp, at END and at MIDDLE.
        let (mut at_end, mut at_mid) = (Vec::new(), Vec::new());
        for &(end, e) in &s.frames {
            if let Some(&(r_end, r_e)) = reference.iter().min_by_key(|r| (r.0 - end).abs()) {
                at_end.push((end - r_end) as f64 / 1e6);
                at_mid.push((mid(end, e) - mid(r_end, r_e)) as f64 / 1e6);
            }
        }
        println!(
            "{:<14} n={:<4} with exposure={:<4} exposure ms min/med/max={:.2}/{:.2}/{:.2}  \
             vs rect_left ms: end {:+.3}  mid {:+.3}",
            s.name,
            s.frames.len(),
            with.len(),
            if with.is_empty() { 0.0 } else { lo },
            median(with.clone()).unwrap_or(0.0),
            hi,
            median(at_end).unwrap_or(f64::NAN),
            median(at_mid).unwrap_or(f64::NAN),
        );
    }
    Ok(())
}
