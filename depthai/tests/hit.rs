//! Hardware integration tests: need an OAK attached and `DEPTHAI_HIT=1`.
//!
//! `DEPTHAI_HIT=1 cargo test -p depthai --test hit -- --ignored --test-threads=1`

use std::time::Duration;

use depthai::node::{Camera, Imu, Node, StereoDepth, Sync, VideoEncoder};
use depthai::{
    CameraBoardSocket, Device, ImgFrame, ImgFrameType, ImgResizeMode, ImuSensor, LengthUnit,
    Message, Pipeline,
};

fn gate() -> bool {
    if std::env::var_os("DEPTHAI_HIT").is_none() {
        eprintln!("DEPTHAI_HIT not set; skipping");
        return false;
    }
    true
}

#[test]
#[ignore]
fn lists_and_opens_a_device() {
    if !gate() {
        return;
    }
    let all = Device::all_available().unwrap();
    assert!(!all.is_empty(), "no OAK found");
    let dev = Device::open(None, None).unwrap();
    assert!(!dev.id().unwrap().is_empty());
    assert!(!dev.connected_cameras().unwrap().is_empty());
    let calib = dev.read_calibration().unwrap();
    let k = calib
        .camera_intrinsics(CameraBoardSocket::CamA, None)
        .unwrap();
    assert_eq!(k[2][2], 1.0);
    let t = calib
        .camera_extrinsics(
            CameraBoardSocket::CamB,
            CameraBoardSocket::CamC,
            false,
            LengthUnit::Meter,
        )
        .unwrap();
    let baseline = (t[0][3].powi(2) + t[1][3].powi(2) + t[2][3].powi(2)).sqrt();
    assert!(
        baseline > 0.01 && baseline < 0.5,
        "baseline {baseline} m looks wrong"
    );
}

#[test]
#[ignore]
fn stereo_sync_delivers_synced_pairs() {
    if !gate() {
        return;
    }
    let dev = Device::open(None, None).unwrap();
    let pipeline = Pipeline::new(&dev).unwrap();
    let left = pipeline.create::<Camera>().unwrap();
    left.build(CameraBoardSocket::CamB).unwrap();
    let right = pipeline.create::<Camera>().unwrap();
    right.build(CameraBoardSocket::CamC).unwrap();
    let gray_output = |cam: &Camera| {
        cam.request_output(
            (640, 400),
            Some(ImgFrameType::Gray8),
            ImgResizeMode::Crop,
            Some(30.0),
            Some(false),
        )
        .unwrap()
    };
    let lo = gray_output(&left);
    let ro = gray_output(&right);
    let sync = pipeline.create::<Sync>().unwrap();
    sync.set_sync_threshold(Duration::from_millis(16)).unwrap();
    lo.link(&sync.input("left").unwrap()).unwrap();
    ro.link(&sync.input("right").unwrap()).unwrap();
    let q = sync.out().unwrap().create_output_queue(4, false).unwrap();
    pipeline.start().unwrap();

    let mut got = 0;
    let mut retained: Option<ImgFrame> = None;
    for _ in 0..40 {
        let Some(g) = q.get(Duration::from_secs(2)).unwrap() else {
            continue;
        };
        let l: ImgFrame = g.get("left").unwrap().unwrap();
        let r: ImgFrame = g.get("right").unwrap().unwrap();
        assert_eq!(l.img_type(), ImgFrameType::Gray8);
        assert_eq!(l.data().len(), 640 * 400);
        assert_eq!(r.data().len(), 640 * 400);
        assert!(g.is_synced(Duration::from_millis(16)).unwrap());
        let now = depthai::steady_now().unwrap();
        assert!(l.timestamp() <= now, "frame timestamp in the future");
        if retained.is_none() {
            retained = Some(l.clone());
        }
        got += 1;
        if got >= 10 {
            break;
        }
    }
    assert!(got >= 10, "only {got} pairs");
    // A retained frame stays readable after many later polls.
    let r = retained.unwrap();
    assert_eq!(r.data().len(), 640 * 400);
    pipeline.stop().unwrap();
}

#[test]
#[ignore]
fn imu_streams_nonzero_timestamps() {
    if !gate() {
        return;
    }
    let dev = Device::open(None, None).unwrap();
    let name = dev.connected_imu().unwrap();
    if name.is_empty() || name == "NONE" {
        eprintln!("no IMU on this board; skipping");
        return;
    }
    let pipeline = Pipeline::new(&dev).unwrap();
    let imu = pipeline.create::<Imu>().unwrap();
    imu.enable_sensor(ImuSensor::AccelerometerRaw, 100).unwrap();
    imu.enable_sensor(ImuSensor::GyroscopeRaw, 100).unwrap();
    imu.set_batch_report_threshold(5).unwrap();
    imu.set_max_batch_reports(5).unwrap();
    let q = imu.out().unwrap().create_output_queue(50, false).unwrap();
    pipeline.start().unwrap();
    let mut samples = 0;
    for _ in 0..50 {
        let Some(b) = q.get(Duration::from_millis(500)).unwrap() else {
            continue;
        };
        for p in b.packets().unwrap() {
            if !p.accelerometer.timestamp.is_zero() && !p.gyroscope.timestamp.is_zero() {
                samples += 1;
            }
        }
        if samples > 50 {
            break;
        }
    }
    assert!(samples > 50, "only {samples} IMU samples");
    pipeline.stop().unwrap();
}

/// Device nodes cannot be created on a host-only pipeline, so the fixed port
/// names the wrappers assume are pinned here, against a real device.
#[test]
#[ignore]
fn device_node_port_names_are_what_the_wrappers_assume() {
    if !gate() {
        return;
    }
    let dev = Device::open(None, None).unwrap();
    let p = Pipeline::new(&dev).unwrap();
    let enc = p.create::<VideoEncoder>().unwrap();
    enc.input().unwrap();
    enc.bitstream().unwrap();
    enc.out().unwrap();
    let sd = p.create::<StereoDepth>().unwrap();
    for name in ["left", "right", "inputAlignTo"] {
        assert!(
            sd.input_by_name(name).unwrap().is_some(),
            "StereoDepth input {name}"
        );
    }
    for name in [
        "depth",
        "disparity",
        "rectifiedLeft",
        "rectifiedRight",
        "syncedLeft",
        "syncedRight",
        "confidenceMap",
    ] {
        assert!(
            sd.output_by_name(name).unwrap().is_some(),
            "StereoDepth output {name}"
        );
    }
    let imu = p.create::<Imu>().unwrap();
    imu.out().unwrap();
    assert_eq!(imu.type_name().unwrap(), "IMU");

    // Node::Output* handed out earlier must still be valid after more requests.
    let cam = p.create::<Camera>().unwrap();
    cam.build(CameraBoardSocket::CamA).unwrap();
    let first = cam
        .request_output((640, 400), None, ImgResizeMode::Crop, None, None)
        .unwrap();
    let name0 = first.name().unwrap();
    for _ in 0..3 {
        cam.request_output((320, 200), None, ImgResizeMode::Crop, None, None)
            .unwrap();
    }
    assert_eq!(first.name().unwrap(), name0);
}

/// The Gate valve: closed → no frames reach the host; open → frames flow again.
#[test]
#[ignore]
fn gate_closes_and_opens_a_stream() {
    if !gate() {
        return;
    }
    use depthai::node::Gate;
    use depthai::GateControl;
    let dev = Device::open(None, None).unwrap();
    let pipeline = Pipeline::new(&dev).unwrap();
    let cam = pipeline.create::<Camera>().unwrap();
    cam.build(CameraBoardSocket::CamA).unwrap();
    let out = cam
        .request_output(
            (320, 240),
            Some(ImgFrameType::Nv12),
            ImgResizeMode::Crop,
            Some(30.0),
            None,
        )
        .unwrap();
    let g = pipeline.create::<Gate>().unwrap();
    out.link(&g.input().unwrap()).unwrap();
    let q = g
        .output()
        .unwrap()
        .cast::<ImgFrame>()
        .create_output_queue(4, false)
        .unwrap();
    let ctl = g
        .input_control()
        .unwrap()
        .create_input_queue(4, false)
        .unwrap();
    pipeline.start().unwrap();
    assert!(
        q.get(Duration::from_secs(3)).unwrap().is_some(),
        "open gate delivers"
    );
    ctl.send(&GateControl::close().unwrap()).unwrap();
    std::thread::sleep(Duration::from_millis(500)); // in-flight frames land
    while q.try_get().unwrap().is_some() {}
    assert!(
        q.get(Duration::from_secs(1)).unwrap().is_none(),
        "closed gate delivers nothing"
    );
    ctl.send(&GateControl::open().unwrap()).unwrap();
    assert!(
        q.get(Duration::from_secs(3)).unwrap().is_some(),
        "reopened gate delivers"
    );
    pipeline.stop().unwrap();
}

/// DetectionNetwork on an RVC2: the zoo YOLO model builds, runs, and produces
/// an ImgDetections (first run downloads the archive; needs internet).
#[test]
#[ignore]
fn detection_network_runs_zoo_yolo() {
    if !gate() {
        return;
    }
    use depthai::node::DetectionNetwork;
    use depthai::NNModelDescription;
    let dev = Device::open(None, None).unwrap();
    let pipeline = Pipeline::new(&dev).unwrap();
    let cam = pipeline.create::<Camera>().unwrap();
    cam.build(CameraBoardSocket::CamA).unwrap();
    let det = pipeline.create::<DetectionNetwork>().unwrap();
    det.build_camera(
        &cam,
        &NNModelDescription::new("luxonis/yolov6-nano:r2-coco-512x288"),
        Some(10.0),
        None,
    )
    .unwrap();
    assert_eq!(
        det.classes().unwrap().len(),
        80,
        "COCO class list from the archive"
    );
    let q = det.out().unwrap().create_output_queue(4, false).unwrap();
    let raw = det
        .out_network()
        .unwrap()
        .create_output_queue(4, false)
        .unwrap();
    pipeline.start().unwrap();
    let dets = q
        .get(Duration::from_secs(15))
        .unwrap()
        .expect("an ImgDetections within 15 s");
    let _ = dets.detections().unwrap(); // may be empty; must decode
    let nn = raw
        .get(Duration::from_secs(5))
        .unwrap()
        .expect("NNData on outNetwork");
    let tensors = nn.tensors().unwrap();
    assert!(!tensors.is_empty(), "YOLO reports its output layers");
    for t in &tensors {
        let n = nn.tensor_f32(t, true).unwrap().len();
        assert_eq!(n, t.num_elements(), "{}: dims {:?}", t.name, t.dims);
    }
    pipeline.stop().unwrap();
}
