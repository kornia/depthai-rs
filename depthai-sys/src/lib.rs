//! Raw FFI to Luxonis depthai-core v3 through a hand-written pure-C shim
//! (`csrc/depthai_c.h`), compiled and linked by this crate's `build.rs`.
//!
//! Everything here is `unsafe` and pointer-shaped; the safe API is the
//! [`depthai`](https://crates.io/crates/depthai) crate. No C++ is visible to
//! Rust, no bindgen/autocxx runs at build time: the ABI is the C header, and the
//! declarations in [`ffi`] are generated from it by `scripts/gen_ffi.py`.
//!
//! Conventions (see the header for the full contract):
//! - `int` returns: [`DAI_OK`] / [`DAI_ERR`]; poll-style functions return
//!   `1` (got) / `0` (none) / `-1` (error).
//! - On `-1`, [`ffi::dai_last_error`] holds a thread-local message; read it on
//!   the calling thread right away.
//! - Refcounted handles are released with the matching `dai_*_release`.
//!   `dai_output` / `dai_input` are borrowed from their node.
//! - `char**` out-strings are freed with [`ffi::dai_string_free`].

#![allow(non_camel_case_types)]

pub mod ffi;
pub use ffi::*;

pub const DAI_OK: i32 = 0;
pub const DAI_ERR: i32 = -1;

/// The depthai-core tag this crate's constants were pinned against.
pub const DEPTHAI_CORE_TAG: &str = "v3.7.1";

// ---------------------------------------------------------------------------
// Opaque handles
// ---------------------------------------------------------------------------
macro_rules! opaque {
    ($($name:ident),* $(,)?) => {$(
        #[repr(C)]
        pub struct $name {
            _private: [u8; 0],
            _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
        }
    )*};
}
opaque!(
    dai_device,
    dai_pipeline,
    dai_node,
    dai_output,
    dai_input,
    dai_queue,
    dai_msg,
    dai_calib,
    dai_bootloader
);

// ---------------------------------------------------------------------------
// Enum constants (mirrors of the header; static_assert'ed against dai:: there)
// ---------------------------------------------------------------------------
pub const DAI_CAM_AUTO: i32 = -1;
pub const DAI_CAM_A: i32 = 0;
pub const DAI_CAM_B: i32 = 1;
pub const DAI_CAM_C: i32 = 2;
pub const DAI_CAM_D: i32 = 3;
pub const DAI_CAM_E: i32 = 4;
pub const DAI_CAM_F: i32 = 5;
pub const DAI_CAM_G: i32 = 6;
pub const DAI_CAM_H: i32 = 7;

pub const DAI_USB_UNKNOWN: i32 = 0;
pub const DAI_USB_LOW: i32 = 1;
pub const DAI_USB_FULL: i32 = 2;
pub const DAI_USB_HIGH: i32 = 3;
pub const DAI_USB_SUPER: i32 = 4;
pub const DAI_USB_SUPER_PLUS: i32 = 5;

pub const DAI_RESIZE_CROP: i32 = 0;
pub const DAI_RESIZE_STRETCH: i32 = 1;
pub const DAI_RESIZE_LETTERBOX: i32 = 2;

pub const DAI_IMG_YUV420P: i32 = 2;
pub const DAI_IMG_RGB888P: i32 = 7;
pub const DAI_IMG_BGR888P: i32 = 8;
pub const DAI_IMG_RGB888I: i32 = 9;
pub const DAI_IMG_BGR888I: i32 = 10;
pub const DAI_IMG_RAW16: i32 = 14;
pub const DAI_IMG_RAW8: i32 = 18;
pub const DAI_IMG_NV12: i32 = 22;
pub const DAI_IMG_BITSTREAM: i32 = 24;
pub const DAI_IMG_GRAY8: i32 = 30;
pub const DAI_IMG_NONE: i32 = 33;

pub const DAI_DT_ADATATYPE: i32 = 0;
pub const DAI_DT_BUFFER: i32 = 1;
pub const DAI_DT_IMG_FRAME: i32 = 2;
pub const DAI_DT_ENCODED_FRAME: i32 = 3;
pub const DAI_DT_IMU_DATA: i32 = 19;
pub const DAI_DT_MESSAGE_GROUP: i32 = 28;

pub const DAI_CAMERA_MODEL_PERSPECTIVE: i32 = 0;
pub const DAI_CAMERA_MODEL_FISHEYE: i32 = 1;
pub const DAI_CAMERA_MODEL_EQUIRECTANGULAR: i32 = 2;
pub const DAI_CAMERA_MODEL_RADIAL_DIVISION: i32 = 3;

pub const DAI_LENGTH_METER: i32 = 0;
pub const DAI_LENGTH_CENTIMETER: i32 = 1;
pub const DAI_LENGTH_MILLIMETER: i32 = 2;
pub const DAI_LENGTH_INCH: i32 = 3;
pub const DAI_LENGTH_FOOT: i32 = 4;
pub const DAI_LENGTH_CUSTOM: i32 = 5;

pub const DAI_IMU_ACCELEROMETER_RAW: i32 = 0x14;
pub const DAI_IMU_ACCELEROMETER_CALIBRATED: i32 = 0x01;
pub const DAI_IMU_GYROSCOPE_RAW: i32 = 0x15;
pub const DAI_IMU_MAGNETOMETER_RAW: i32 = 0x16;
pub const DAI_IMU_ROTATION_VECTOR: i32 = 0x05;

pub const DAI_IMU_ACCURACY_UNRELIABLE: i32 = 0;
pub const DAI_IMU_ACCURACY_LOW: i32 = 1;
pub const DAI_IMU_ACCURACY_MEDIUM: i32 = 2;
pub const DAI_IMU_ACCURACY_HIGH: i32 = 3;

pub const DAI_VENC_H264_BASELINE: i32 = 0;
pub const DAI_VENC_H264_HIGH: i32 = 1;
pub const DAI_VENC_H264_MAIN: i32 = 2;
pub const DAI_VENC_H265_MAIN: i32 = 3;
pub const DAI_VENC_MJPEG: i32 = 4;
pub const DAI_VENC_RC_CBR: i32 = 0;
pub const DAI_VENC_RC_VBR: i32 = 1;

pub const DAI_STEREO_PRESET_FAST_ACCURACY: i32 = 0;
pub const DAI_STEREO_PRESET_FAST_DENSITY: i32 = 1;
pub const DAI_STEREO_PRESET_DEFAULT: i32 = 2;
pub const DAI_STEREO_PRESET_FACE: i32 = 3;
pub const DAI_STEREO_PRESET_HIGH_DETAIL: i32 = 4;
pub const DAI_STEREO_PRESET_ROBOTICS: i32 = 5;
pub const DAI_STEREO_PRESET_DENSITY: i32 = 6;
pub const DAI_STEREO_PRESET_ACCURACY: i32 = 7;

pub const DAI_XLINK_STATE_ANY: i32 = 0;
pub const DAI_XLINK_STATE_BOOTED: i32 = 1;
pub const DAI_XLINK_STATE_UNBOOTED: i32 = 2;
pub const DAI_XLINK_STATE_BOOTLOADER: i32 = 3;
pub const DAI_XLINK_STATE_FLASH_BOOTED: i32 = 4;

pub const DAI_PLATFORM_RVC2: i32 = 0;
pub const DAI_PLATFORM_RVC3: i32 = 1;
pub const DAI_PLATFORM_RVC4: i32 = 2;

// ---------------------------------------------------------------------------
// PODs — field order and types must match csrc/depthai_c.h exactly.
// ---------------------------------------------------------------------------
/// One entry of `dai::Device::getAllAvailableDevices()`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct dai_device_info {
    pub name: [u8; 64],
    pub device_id: [u8; 64],
    pub state: i32,
    pub protocol: i32,
    pub platform: i32,
    pub status: i32,
    pub reserved_: [i32; 2],
}

impl Default for dai_device_info {
    fn default() -> Self {
        // SAFETY: all-zero is a valid value for every field (u8/i32 arrays and ints).
        unsafe { core::mem::zeroed() }
    }
}

/// Everything about a `dai::ImgFrame` except its pixels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct dai_img_frame_info {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub type_: i32,
    pub instance_num: u32,
    pub pad_: u32,
    pub sequence_num: i64,
    pub timestamp_ns: i64,
    pub timestamp_device_ns: i64,
    pub data_len: usize,
}

/// One `dai::IMUReport` with x/y/z.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct dai_imu_vec_report {
    pub ts_sec: i64,
    pub ts_nsec: i64,
    pub ts_device_sec: i64,
    pub ts_device_nsec: i64,
    pub sequence: i32,
    pub accuracy: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub pad_: f32,
}

/// `dai::IMUReportRotationVectorWAcc`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct dai_imu_rotvec_report {
    pub ts_sec: i64,
    pub ts_nsec: i64,
    pub ts_device_sec: i64,
    pub ts_device_nsec: i64,
    pub sequence: i32,
    pub accuracy: i32,
    pub i: f32,
    pub j: f32,
    pub k: f32,
    pub real: f32,
    pub accuracy_rad: f32,
    pub pad_: f32,
}

/// `dai::IMUPacket`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct dai_imu_packet {
    pub accelerometer: dai_imu_vec_report,
    pub gyroscope: dai_imu_vec_report,
    pub magnetic_field: dai_imu_vec_report,
    pub rotation_vector: dai_imu_rotvec_report,
}

/// `dai::EncodedFrame` metadata.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct dai_encoded_frame_info {
    pub width: u32,
    pub height: u32,
    pub profile: i32,
    pub frame_type: i32,
    pub quality: u32,
    pub bitrate: u32,
    pub lossless: i32,
    pub instance_num: u32,
    pub sequence_num: i64,
    pub timestamp_ns: i64,
    pub data_len: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    // The C side pins the same numbers with static_assert; a mismatch here means the
    // two declarations drifted apart.
    #[test]
    fn pod_layouts_match_header() {
        assert_eq!(size_of::<dai_device_info>(), 152);
        assert_eq!(size_of::<dai_img_frame_info>(), 56);
        assert_eq!(size_of::<dai_imu_vec_report>(), 56);
        assert_eq!(size_of::<dai_imu_rotvec_report>(), 64);
        assert_eq!(size_of::<dai_imu_packet>(), 232);
        assert_eq!(size_of::<dai_encoded_frame_info>(), 56);
        assert_eq!(align_of::<dai_img_frame_info>(), 8);
        assert_eq!(align_of::<dai_imu_packet>(), 8);
    }

    #[test]
    fn last_error_is_never_null() {
        let p = unsafe { dai_last_error() };
        assert!(!p.is_null());
    }

    #[test]
    fn steady_clock_is_monotone_or_stubbed() {
        let mut a = 0i64;
        let mut b = 0i64;
        let ra = unsafe { dai_steady_clock_now_ns(&mut a) };
        let rb = unsafe { dai_steady_clock_now_ns(&mut b) };
        if ra == DAI_OK && rb == DAI_OK {
            assert!(b >= a);
        } else {
            // Stub build (DEPTHAI_SYS_SKIP_NATIVE): every call errors with a message.
            let msg = unsafe { std::ffi::CStr::from_ptr(dai_last_error()) };
            assert!(!msg.to_bytes().is_empty());
        }
    }
}
