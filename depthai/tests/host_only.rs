//! Graph-construction tests that need depthai-core but NO camera
//! (`Pipeline::host_only()`). Only host-capable nodes (Sync) can be created
//! there — depthai refuses device nodes on a host-only pipeline, so the device
//! nodes' port names are pinned in `hit.rs` instead. These skip themselves when
//! the crate was built with `DEPTHAI_SYS_SKIP_NATIVE=1` (every call errors then).

use depthai::node::{Node, Sync};
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
fn unknown_port_is_none_not_error() {
    let Some(p) = host_pipeline() else { return };
    // Only host-capable nodes exist without a device; Sync is one.
    let sync = p.create::<Sync>().unwrap();
    assert!(sync.output_by_name("nope").unwrap().is_none());
    assert!(sync.input_by_name("nope").unwrap().is_none());
    assert_eq!(sync.type_name().unwrap(), "Sync");
    assert!(sync
        .output_names()
        .unwrap()
        .iter()
        .any(|n| n.ends_with("/out")));
}
