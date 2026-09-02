//! On-device object detection on an OAK-D (RVC2): `DetectionNetwork` running
//! YOLOv6-nano from the Luxonis model zoo, decoded on the device into
//! `ImgDetections`. The first run downloads the archive (internet needed) into
//! `.depthai_cached_models/` (or `DEPTHAI_ZOO_CACHE_PATH`).
//!
//! `cargo run --example yolo_detect [-- <device-id-or-ip>]`

use std::time::{Duration, Instant};

use depthai::node::{Camera, DetectionNetwork};
use depthai::{CameraBoardSocket, Device, Message, NNModelDescription, Pipeline};

fn main() -> depthai::Result<()> {
    let id = std::env::args().nth(1);
    let dev = Device::open(id.as_deref(), None)?;
    let pipeline = Pipeline::new(&dev)?;

    let cam = pipeline.create::<Camera>()?;
    cam.build(CameraBoardSocket::CamA)?;

    let det = pipeline.create::<DetectionNetwork>()?;
    // Fetches the RVC2/RVC4 variant for the connected device and requests a
    // matching 512x288 camera output.
    det.build_camera(
        &cam,
        &NNModelDescription::new("luxonis/yolov6-nano:r2-coco-512x288"),
        Some(15.0),
        None,
    )?;
    det.set_confidence_threshold(0.5)?;
    let classes = det.classes()?;
    let q = det.out()?.create_output_queue(4, false)?;

    pipeline.start()?;
    println!("{} classes; detecting for 10 s:", classes.len());
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        let Some(dets) = q.get(Duration::from_secs(1))? else {
            continue;
        };
        let list = dets.detections()?;
        println!("#{:<5} {} detections", dets.sequence_num(), list.len());
        for d in &list {
            println!(
                "    {:<12} {:.2}  [{:.2} {:.2} {:.2} {:.2}]",
                if d.label_name.is_empty() {
                    d.label.to_string()
                } else {
                    d.label_name.clone()
                },
                d.confidence,
                d.xmin,
                d.ymin,
                d.xmax,
                d.ymax
            );
        }
    }
    pipeline.stop()?;
    Ok(())
}
