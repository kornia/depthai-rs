# depthai-rs

Safe Rust for Luxonis **depthai-core v3** (OAK cameras), with no C++ visible to
Rust and no `bindgen`/`autocxx`/`cxx` at build time.

| Crate | Role |
|-------|------|
| `depthai-sys` | Raw FFI. A hand-written **pure-C shim** (`csrc/depthai_c.h/.cpp`, ~120 functions) over depthai-core, compiled by `build.rs`; `links = "depthai-core"`. Enum values and struct layouts are `static_assert`-pinned against the pinned depthai-core tag. |
| `depthai` | The safe wrapper: `Device`, `Pipeline`, nodes (`Camera`, `Sync`, `StereoDepth`, `VideoEncoder`, `Imu`), typed `Output<M>` / `OutputQueue<M>`, messages (`ImgFrame`, `MessageGroup`, `ImuData`, `EncodedFrame`), `CalibrationHandler`, `DeviceBootloader`. |

Pinned depthai-core: **v3.7.1**.

`depthai-sys` is **raw**: one C function per depthai-core member, named after
it; `std::optional`/overloads collapse into sentinels that select the C++
default. The wrapper is deliberately **faithful and unopinionated**: it does not read
environment variables, convert timestamps to wall-clock, clamp IMU rates,
pick stereo presets, repack strides, or default the unit / spec-translation
choices in the calibration getters. Those decisions belong to the driver built
on top (see [`kornia/sensor-rt`](https://github.com/kornia/sensor-rt)'s
`sensor-oak`).

```rust
use std::time::Duration;
use depthai::node::{Camera, Sync};
use depthai::{CameraBoardSocket, Device, ImgFrame, ImgFrameType, ImgResizeMode, Message, Pipeline};

let dev = Device::open(None, None)?;                 // first available OAK
let pipeline = Pipeline::new(&dev)?;
let left = pipeline.create::<Camera>()?;  left.build(CameraBoardSocket::CamB)?;
let right = pipeline.create::<Camera>()?; right.build(CameraBoardSocket::CamC)?;
let lo = left.request_output((640, 400), Some(ImgFrameType::Gray8), ImgResizeMode::Crop, Some(30.0), Some(false))?;
let ro = right.request_output((640, 400), Some(ImgFrameType::Gray8), ImgResizeMode::Crop, Some(30.0), Some(false))?;
let sync = pipeline.create::<Sync>()?;
sync.set_sync_threshold(Duration::from_millis(16))?;
lo.link(&sync.input("left")?)?;
ro.link(&sync.input("right")?)?;
let q = sync.out()?.create_output_queue(4, false)?;
pipeline.start()?;
while let Some(group) = q.get(Duration::from_secs(1))? {
    let l: ImgFrame = group.get("left")?.expect("left");
    let r: ImgFrame = group.get("right")?.expect("right");
    // l.data() / r.data(): zero-copy GRAY8; clone the frame to keep it past the next poll
}
```

Messages are refcounted handles: `Clone` is a refcount bump, `data()` is
zero-copy, and a clone stays valid across later polls and threads
(`Send + Sync`).

## Building

`depthai-sys` needs a depthai-core **install prefix** (`lib/cmake/depthai`,
`include/depthai`, `lib/libdepthai-core.so`). depthai-core is not packaged for
Ubuntu/Debian; Luxonis ships only pip wheels and source. The ROS apt repos carry
`ros-kilted-depthai` (3.9.0, Ubuntu 24.04 only) — `ros-humble-depthai` on 22.04
is v2 and unusable here.

Resolution order in `build.rs`:

1. `DEPTHAI_PREFIX=/path/to/prefix`
2. a `vendor/depthai` directory found by walking up from the crate — what
   `depthai-sys/scripts/build_depthai.sh` produces from the pinned
   `depthai-sys/vendor/depthai-core` submodule:
   ```bash
   git submodule update --init --recursive
   bash depthai-sys/scripts/build_depthai.sh     # 10-30 min, RAM-capped (-j2 under 16 GB)
   ```
3. with `--features vendored`: the same build, done by `build.rs` into
   `~/.cache/kornia-depthai/<tag>/<target>` (`DEPTHAI_RS_CACHE_DIR`,
   `DEPTHAI_JOBS` to override).

`libusb-1.0` is a runtime dependency of depthai-core; `build.rs` finds it via
`pkg-config` (or `DEPTHAI_LIBUSB_DIR`). The absolute `rpath` of the prefix is
baked into binaries.

**Check-only builds** (CI, laptops without depthai-core):
`DEPTHAI_SYS_SKIP_NATIVE=1 cargo test` links an error-only stub so
`cargo check`/`clippy` and the pure-Rust tests run anywhere. Never ship that.

On a Jetson Orin cap cargo with `CARGO_BUILD_JOBS=2`.

## Tests

- `cargo test` — unit tests (layout pins, enums, conversions) + host-only graph
  tests (`Pipeline::host_only()`, no camera; auto-skip on the stub).
- `DEPTHAI_HIT=1 cargo test -p depthai --test hit -- --ignored --test-threads=1`
  — hardware tests with an OAK attached.
- `cargo run --example list_devices | stereo_sync | imu_dump | rgbd_depth`.

## Adding to the ABI

1. Declare the function in `depthai-sys/csrc/depthai_c.h`, implement it in
   `depthai_c.cpp` (wrap the body in `DAI_GUARD`).
2. `python3 depthai-sys/scripts/gen_ffi.py && python3 depthai-sys/scripts/gen_stub.py`
   (regenerates `src/ffi.rs` and the stub; CI checks they are committed).
3. Wrap it in `depthai/`.

Symbol prefix is `dai_`; if you ever link this next to another depthai binding
that also uses `dai_`, expect duplicate symbols.

## License

Apache-2.0.
