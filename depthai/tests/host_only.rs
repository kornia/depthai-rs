//! Graph-construction tests that need depthai-core but NO camera
//! (`Pipeline::host_only()`). They skip themselves when the crate was built with
//! `DEPTHAI_SYS_SKIP_NATIVE=1` (every call errors then).

use depthai::node::{Camera, Imu, Node, StereoDepth, Sync, VideoEncoder};
use depthai::Pipeline;

fn host_pipeline() -> Option<Pipeline> {
    match Pipeline::host_only() {
        Ok(p) => Some(p),
        Err(e) if e.to_string().contains("DEPTHAI_SYS_SKIP_NATIVE") => {
            eprintln!("skipped: {e}");
            None
        }
        Err(e) => panic!("host-only pipeline: {e}"),
    }
}

#[test]
fn fixed_port_names_are_what_the_wrappers_assume() {
    let Some(p) = host_pipeline() else { return };
    let enc = p.create::<VideoEncoder>().unwrap();
    assert!(enc
        .input_names()
        .unwrap()
        .iter()
        .any(|n| n.ends_with("/in")));
    assert!(enc
        .output_names()
        .unwrap()
        .iter()
        .any(|n| n.ends_with("/bitstream")));
    assert!(enc
        .output_names()
        .unwrap()
        .iter()
        .any(|n| n.ends_with("/out")));
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

    let sync = p.create::<Sync>().unwrap();
    sync.out().unwrap();
}

#[test]
fn sync_input_map_is_get_or_create_and_stable() {
    let Some(p) = host_pipeline() else { return };
    let sync = p.create::<Sync>().unwrap();
    let a = sync.input("left").unwrap();
    let b = sync.input("right").unwrap();
    let a2 = sync.input("left").unwrap();
    assert!(a.ptr_eq(&a2), "same key must return the same port");
    assert!(!a.ptr_eq(&b));
    let names = sync.input_names().unwrap();
    assert!(names.iter().any(|n| n.ends_with("/left")));
}

#[test]
fn camera_outputs_survive_later_requests() {
    let Some(p) = host_pipeline() else { return };
    let cam = p.create::<Camera>().unwrap();
    if cam.build(depthai::CameraBoardSocket::CamA).is_err() {
        eprintln!("Camera::build needs a device in this depthai version; skipping");
        return;
    }
    let first = cam
        .request_output((640, 400), None, depthai::ImgResizeMode::Crop, None, None)
        .unwrap();
    let name0 = first.name().unwrap();
    for _ in 0..3 {
        cam.request_output((320, 200), None, depthai::ImgResizeMode::Crop, None, None)
            .unwrap();
    }
    // Node::Output* handed out earlier must still be valid after more requests.
    assert_eq!(first.name().unwrap(), name0);
}

#[test]
fn unknown_port_is_none_not_error() {
    let Some(p) = host_pipeline() else { return };
    let imu = p.create::<Imu>().unwrap();
    assert!(imu.output_by_name("nope").unwrap().is_none());
    assert!(imu.input_by_name("nope").unwrap().is_none());
    assert_eq!(imu.type_name(), "IMU");
}
