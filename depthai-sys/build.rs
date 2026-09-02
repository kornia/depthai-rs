//! Build the pure-C shim (`csrc/depthai_c.cpp`) against depthai-core and link both.
//!
//! Resolution order for the depthai-core prefix (a directory holding
//! `lib/cmake/depthai`, `include/depthai`, `lib/libdepthai-core.so`):
//!
//! 1. `DEPTHAI_PREFIX` (explicit).
//! 2. A `vendor/depthai` directory found by walking up from this crate — what
//!    `scripts/build_depthai.sh` produces.
//! 3. With the `vendored` feature: build the `vendor/depthai-core` submodule via
//!    CMake into a per-tag cache dir (`DEPTHAI_RS_CACHE_DIR`, default
//!    `~/.cache/kornia-depthai/<tag>/<target>`), RAM-capped (see `jobs()`).
//! 4. Otherwise: fail with instructions.
//!
//! `DEPTHAI_SYS_SKIP_NATIVE=1` replaces the shim + library with a stub whose every
//! call returns an error. It exists so `cargo check`, `cargo clippy` and pure-Rust
//! unit tests work on machines without depthai-core (CI, laptops). Never ship it.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg_attr(not(feature = "vendored"), allow(dead_code))]
const DEPTHAI_TAG: &str = "v3.7.1";

fn env(name: &str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={name}");
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_prefix(p: &Path) -> bool {
    p.join("lib/cmake/depthai").is_dir()
}

/// Walk up from the crate for a `vendor/depthai` prefix (this crate's own, or a
/// workspace that embeds us).
fn find_vendor_prefix() -> Option<PathBuf> {
    let mut dir = manifest_dir();
    loop {
        let cand = dir.join("vendor/depthai");
        if is_prefix(&cand) {
            return Some(cand);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Parallelism for the depthai-core build is RAM-bound, not core-bound: its
/// template-heavy TUs peak at ~2 GB each in cc1plus. Keep `jobs * 2 GB` under
/// physical RAM (a 7.4 GB Jetson Orin Nano OOMs at -j4). Override with
/// `DEPTHAI_JOBS`.
#[cfg_attr(not(feature = "vendored"), allow(dead_code))]
fn jobs() -> usize {
    if let Some(j) = env("DEPTHAI_JOBS").and_then(|s| s.parse().ok()) {
        return j;
    }
    let mem_gb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|kb| kb.parse::<u64>().ok())
        })
        .map(|kb| kb / 1024 / 1024)
        .unwrap_or(8);
    if mem_gb >= 16 {
        4
    } else {
        2
    }
}

#[cfg_attr(not(feature = "vendored"), allow(dead_code))]
fn cache_root() -> PathBuf {
    if let Some(p) = env("DEPTHAI_RS_CACHE_DIR") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".cache/kornia-depthai")
}

/// Build depthai-core from the submodule into the cache and return the prefix.
#[cfg(feature = "vendored")]
fn build_vendored() -> PathBuf {
    let src = manifest_dir().join("vendor/depthai-core");
    if !src.join("CMakeLists.txt").exists() {
        panic!(
            "depthai-sys[vendored]: {} is empty. Run `git submodule update --init --recursive` \
             in the depthai-rs checkout (build scripts never run git).",
            src.display()
        );
    }
    let target = std::env::var("TARGET").unwrap_or_default();
    let prefix = cache_root().join(DEPTHAI_TAG).join(&target);
    let stamp = prefix.join(".build-stamp");
    if is_prefix(&prefix) && std::fs::read_to_string(&stamp).ok().as_deref() == Some(DEPTHAI_TAG) {
        return prefix;
    }
    let jobs = jobs();
    println!("cargo:warning=depthai-sys: building depthai-core {DEPTHAI_TAG} into {} with -j{jobs} (10-30 min)", prefix.display());
    std::fs::create_dir_all(&prefix).expect("create cache dir");
    let mut cfg = cmake::Config::new(&src);
    cfg.out_dir(&prefix)
        .profile("Release")
        .define("BUILD_SHARED_LIBS", "ON")
        .define("DEPTHAI_BUILD_EXAMPLES", "OFF")
        .define("DEPTHAI_BUILD_TESTS", "OFF")
        .define("DEPTHAI_BUILD_DOCS", "OFF")
        .define("CMAKE_INSTALL_RPATH", "$ORIGIN")
        .env("CMAKE_BUILD_PARALLEL_LEVEL", jobs.to_string())
        .build_arg(format!("-j{jobs}"));
    if which("ninja") {
        cfg.generator("Ninja");
    }
    let dst = cfg.build();
    std::fs::write(&stamp, DEPTHAI_TAG).expect("write stamp");
    dst
}

#[cfg(not(feature = "vendored"))]
fn build_vendored() -> PathBuf {
    panic!(
        "depthai-sys: depthai-core not found. Either set DEPTHAI_PREFIX=<dir containing \
         lib/cmake/depthai> (build one with depthai-sys/scripts/build_depthai.sh), enable the \
         `vendored` feature to build the pinned submodule, or set DEPTHAI_SYS_SKIP_NATIVE=1 for a \
         check-only build."
    );
}

#[cfg_attr(not(feature = "vendored"), allow(dead_code))]
fn which(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// libusb-1.0 is a DT_NEEDED of libdepthai-core.so: bundled in the prefix by the
/// build script, else from the environment (pkg-config). Put its dir on the link
/// search path and the rpath.
fn find_libusb_dir(prefix: &Path) -> Option<PathBuf> {
    if let Some(d) = env("DEPTHAI_LIBUSB_DIR") {
        return Some(PathBuf::from(d));
    }
    // scripts/build_depthai.sh bundles vcpkg's libusb into the prefix itself.
    if prefix.join("lib/libusb-1.0.so").exists() {
        return Some(prefix.join("lib"));
    }
    let out = Command::new("pkg-config")
        .args(["--variable=libdir", "libusb-1.0"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!dir.is_empty() && Path::new(&dir).is_dir()).then(|| PathBuf::from(dir))
}

/// Does this libdepthai-core.so leave `ImageFilters` undefined (built with
/// DEPTHAI_OPENCV_SUPPORT=OFF)? Decided with `nm -D --undefined-only`; when nm is
/// unavailable, assume it does not (a strong definition then wins anyway).
fn library_lacks_image_filters(prefix: &Path) -> bool {
    let lib = prefix.join("lib/libdepthai-core.so");
    let Ok(out) = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&lib)
        .output()
    else {
        println!("cargo:warning=depthai-sys: nm not found; assuming libdepthai-core.so defines ImageFilters");
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let lacks = text.lines().any(|l| l.contains("ImageFilters"));
    if lacks {
        println!("cargo:warning=depthai-sys: libdepthai-core.so was built without OpenCV; compiling weak ImageFilters/Rectification fallbacks");
    }
    lacks
}

fn build_stub() {
    println!("cargo:warning=depthai-sys: DEPTHAI_SYS_SKIP_NATIVE set — linking the error-only stub, NOT depthai-core");
    println!("cargo:rpath=");
    cc::Build::new()
        .file("csrc/depthai_c_stub.c")
        .include("csrc")
        .flag_if_supported("-Wno-unused-parameter")
        .compile("depthai_c_stub");
    println!("cargo:skip_native=1");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc/depthai_c.h");
    println!("cargo:rerun-if-changed=csrc/depthai_c.cpp");
    println!("cargo:rerun-if-changed=csrc/depthai_c_stub.c");
    println!("cargo:rerun-if-changed=csrc/depthai_nocv_stub.cpp");
    println!("cargo:rerun-if-changed=csrc/CMakeLists.txt");

    if env("DEPTHAI_SYS_SKIP_NATIVE").is_some() {
        build_stub();
        return;
    }

    let prefix = env("DEPTHAI_PREFIX")
        .map(PathBuf::from)
        .or_else(find_vendor_prefix)
        .unwrap_or_else(build_vendored);
    if !is_prefix(&prefix) {
        panic!(
            "depthai-sys: {} does not look like a depthai-core install prefix (no lib/cmake/depthai)",
            prefix.display()
        );
    }

    // 1. The shim, with depthai's include + link flags resolved by its CMake config.
    //    The weak no-OpenCV fallbacks are compiled only when this libdepthai-core.so
    //    actually leaves those symbols undefined, so they can never shadow real ones.
    let nocv = library_lacks_image_filters(&prefix);
    let shim = cmake::Config::new("csrc")
        .profile("Release")
        .define("CMAKE_PREFIX_PATH", prefix.to_string_lossy().as_ref())
        .define("DEPTHAI_C_NOCV_STUB", if nocv { "ON" } else { "OFF" })
        .build();
    println!("cargo:rustc-link-search=native={}/lib", shim.display());
    // whole-archive: the archive also carries WEAK fallbacks for symbols an
    // OpenCV-less libdepthai-core.so leaves undefined (csrc/depthai_nocv_stub.cpp).
    // Nothing references them before the shared library is scanned, so they must
    // be force-included rather than pulled on demand.
    println!("cargo:rustc-link-lib=static:+whole-archive=depthai_c");

    // 2. depthai-core (shared) + the C++ runtime, with an absolute rpath so
    //    binaries find the .so without LD_LIBRARY_PATH.
    println!("cargo:rustc-link-search=native={}/lib", prefix.display());
    println!("cargo:rustc-link-lib=dylib=depthai-core");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib", prefix.display());
    println!("cargo:rustc-link-lib=dylib=stdc++");

    // 3. libusb.
    let mut rpath = vec![format!("{}/lib", prefix.display())];
    match find_libusb_dir(&prefix) {
        Some(usb) => {
            rpath.push(usb.display().to_string());
            println!("cargo:rustc-link-search=native={}", usb.display());
            println!("cargo:rustc-link-lib=dylib=usb-1.0");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", usb.display());
            // libdepthai-core.so's DT_NEEDED names the UNVERSIONED libusb-1.0.so, so the
            // linker must be able to resolve that transitive dependency here too.
            println!("cargo:rustc-link-arg=-Wl,-rpath-link,{}", usb.display());
        }
        None => println!(
            "cargo:warning=depthai-sys: libusb-1.0 not found via pkg-config; set DEPTHAI_LIBUSB_DIR if the link fails"
        ),
    }

    // 4. Metadata for dependents (`DEP_DEPTHAI_CORE_PREFIX`, `DEP_DEPTHAI_CORE_INCLUDE`,
    //    `DEP_DEPTHAI_CORE_RPATH`). `rustc-link-arg` above only reaches THIS package's
    //    targets: a crate that ships binaries/examples/tests must add the rpath itself:
    //        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", env!("DEP_DEPTHAI_CORE_RPATH"));
    //    (needs a direct dependency on depthai-sys). Or set LD_LIBRARY_PATH at run time.
    println!("cargo:prefix={}", prefix.display());
    println!("cargo:include={}/include", prefix.display());
    println!("cargo:rpath={}", rpath.join(":"));
}
