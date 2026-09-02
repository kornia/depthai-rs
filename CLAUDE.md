# depthai-rs

Safe Rust for Luxonis depthai-core v3. Two crates: `depthai-sys` (hand-written
pure-C shim + generated `extern "C"` block) and `depthai` (safe wrapper).

## Hard rules

- **depthai-sys is RAW**: one C function per depthai-core member, named after it
  (`dai_<class>_<member>`); overloads/`std::optional` collapse via sentinels that pick
  the C++ default; `*_get_info` structs are plain copies of getters. No invented
  members (one documented exception: `dai_steady_clock_now_ns`, the clock every
  timestamp is relative to), no policy. The safe crate mirrors depthai-core's own abstractions (node
  graph, typed ports, messages) and nothing else.
- **No autocxx / bindgen / cxx.** The ABI is `depthai-sys/csrc/depthai_c.h`. Every
  function: `int` return (`DAI_OK`/`DAI_ERR`, polls `1/0/-1`), thread-local
  `dai_last_error()`, body wrapped in `DAI_GUARD`. Opaque handles; refcounted ones
  are heap `shared_ptr` copies released by `dai_*_release`.
- **Enum values and POD sizes are pinned** with `static_assert` in
  `depthai_c.cpp` and `size_of` tests in `depthai-sys/src/lib.rs`. Bumping
  depthai-core (`DEPTHAI_TAG` in `build.rs`, `TAG` in `scripts/build_depthai.sh`,
  the submodule) is a deliberate edit; the pins tell you what moved.
- **Generated files are committed**: after editing the header run
  `python3 depthai-sys/scripts/gen_ffi.py && python3 depthai-sys/scripts/gen_stub.py`.
  CI diffs them.
- **The safe crate carries no policy.** No env vars, no clock conversion, no rate
  clamps, no default units/spec-translation, no stride fix-ups. Driver crates
  (sensor-oak) own those.
- Every handle is `Send + Sync` with a SAFETY comment saying why. Messages are
  immutable + refcounted; `Clone` = refcount bump; `data()` is zero-copy.
- `#![forbid(unsafe_op_in_unsafe_fn)]` in `depthai`: every FFI call sits in its
  own `unsafe {}` block.

## Building here (Jetson Orin, 7.4 GB RAM)

```bash
export CARGO_BUILD_JOBS=2
export DEPTHAI_PREFIX=/path/to/depthai/prefix   # lib/cmake/depthai inside
cargo build && cargo test
DEPTHAI_HIT=1 cargo test -p depthai --test hit -- --ignored --test-threads=1   # OAK attached
```

No prefix on the machine? `DEPTHAI_SYS_SKIP_NATIVE=1 cargo test` links an
error-only stub (check/clippy/unit tests only). Source build:
`DEPTHAI_CMAKE_EXTRA="-D DEPTHAI_OPENCV_SUPPORT=OFF" bash depthai-sys/scripts/build_depthai.sh`
(~4 GB free for vcpkg build trees, 30-60 min at -j2). OpenCV OFF leaves
`ImageFilters`/`Rectification` symbols undefined in libdepthai-core.so;
`csrc/depthai_nocv_stub.cpp` supplies weak fallbacks — never patch depthai-core. Hardware tests: `DEPTHAI_HIT=1`.

## Consumers

`kornia/sensor-rt` `sensor-oak` pins this repo by git rev/tag. Keep the public
API additive; sensor-oak's public API must not have to change for a bump.
