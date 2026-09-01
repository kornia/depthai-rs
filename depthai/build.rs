//! Bake the depthai-core (and libusb) directories into the rpath of this crate's
//! examples and tests. `cargo:rustc-link-arg` from depthai-sys's build script only
//! applies to depthai-sys's own targets, so every crate with binaries repeats
//! this one line using the metadata depthai-sys exports.
fn main() {
    println!("cargo:rerun-if-env-changed=DEP_DEPTHAI_CORE_RPATH");
    if let Ok(rpath) = std::env::var("DEP_DEPTHAI_CORE_RPATH") {
        for dir in rpath.split(':').filter(|d| !d.is_empty()) {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
    }
}
