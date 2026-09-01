//! Enumerations mirrored from depthai-core.
//!
//! Every enum carries an `Other(i32)` arm: values the device reports that this
//! crate does not name are passed through instead of turned into an error, and
//! you can pass raw depthai values the crate does not name yet. The numeric
//! values are pinned against depthai-core by `static_assert`s in the C shim.

use depthai_sys as sys;

macro_rules! dai_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident = $konst:ident ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )*
            /// A value this crate does not name; carried through verbatim.
            Other(i32),
        }

        impl $name {
            /// The depthai-core numeric value.
            pub const fn to_raw(self) -> i32 {
                match self {
                    $( Self::$variant => sys::$konst, )*
                    Self::Other(v) => v,
                }
            }

            /// From the depthai-core numeric value (never fails).
            pub const fn from_raw(v: i32) -> Self {
                $( if v == sys::$konst { return Self::$variant; } )*
                Self::Other(v)
            }
        }

        impl From<$name> for i32 {
            fn from(v: $name) -> i32 {
                v.to_raw()
            }
        }

        impl From<i32> for $name {
            fn from(v: i32) -> Self {
                Self::from_raw(v)
            }
        }
    };
}

dai_enum! {
    /// `dai::CameraBoardSocket`. On an OAK-D: `CamA` = colour, `CamB` = left mono,
    /// `CamC` = right mono.
    CameraBoardSocket {
        Auto = DAI_CAM_AUTO,
        CamA = DAI_CAM_A,
        CamB = DAI_CAM_B,
        CamC = DAI_CAM_C,
        CamD = DAI_CAM_D,
        CamE = DAI_CAM_E,
        CamF = DAI_CAM_F,
        CamG = DAI_CAM_G,
        CamH = DAI_CAM_H,
    }
}

dai_enum! {
    /// `dai::UsbSpeed`.
    UsbSpeed {
        Unknown = DAI_USB_UNKNOWN,
        Low = DAI_USB_LOW,
        Full = DAI_USB_FULL,
        /// USB 2.
        High = DAI_USB_HIGH,
        /// USB 3.
        Super = DAI_USB_SUPER,
        SuperPlus = DAI_USB_SUPER_PLUS,
    }
}

dai_enum! {
    /// `dai::ImgResizeMode`.
    ImgResizeMode {
        Crop = DAI_RESIZE_CROP,
        Stretch = DAI_RESIZE_STRETCH,
        Letterbox = DAI_RESIZE_LETTERBOX,
    }
}

dai_enum! {
    /// `dai::ImgFrame::Type` (the members this crate names; others pass through).
    ImgFrameType {
        Yuv420p = DAI_IMG_YUV420P,
        Rgb888p = DAI_IMG_RGB888P,
        Bgr888p = DAI_IMG_BGR888P,
        /// Interleaved RGB, 3 bytes per pixel.
        Rgb888i = DAI_IMG_RGB888I,
        Bgr888i = DAI_IMG_BGR888I,
        /// 16-bit raw / depth in millimetres.
        Raw16 = DAI_IMG_RAW16,
        Raw8 = DAI_IMG_RAW8,
        Nv12 = DAI_IMG_NV12,
        /// Encoder output (H.264/H.265/MJPEG bytes).
        Bitstream = DAI_IMG_BITSTREAM,
        /// 8-bit grayscale, 1 byte per pixel.
        Gray8 = DAI_IMG_GRAY8,
        None = DAI_IMG_NONE,
    }
}

dai_enum! {
    /// `dai::DatatypeEnum` (the members this crate names).
    Datatype {
        ADatatype = DAI_DT_ADATATYPE,
        Buffer = DAI_DT_BUFFER,
        ImgFrame = DAI_DT_IMG_FRAME,
        EncodedFrame = DAI_DT_ENCODED_FRAME,
        ImuData = DAI_DT_IMU_DATA,
        MessageGroup = DAI_DT_MESSAGE_GROUP,
    }
}

dai_enum! {
    /// `dai::CameraModel` — the lens model a calibration was fitted with.
    CameraModel {
        Perspective = DAI_CAMERA_MODEL_PERSPECTIVE,
        Fisheye = DAI_CAMERA_MODEL_FISHEYE,
        Equirectangular = DAI_CAMERA_MODEL_EQUIRECTANGULAR,
        RadialDivision = DAI_CAMERA_MODEL_RADIAL_DIVISION,
    }
}

dai_enum! {
    /// `dai::LengthUnit` for extrinsic translations.
    LengthUnit {
        Meter = DAI_LENGTH_METER,
        Centimeter = DAI_LENGTH_CENTIMETER,
        Millimeter = DAI_LENGTH_MILLIMETER,
        Inch = DAI_LENGTH_INCH,
        Foot = DAI_LENGTH_FOOT,
        Custom = DAI_LENGTH_CUSTOM,
    }
}

dai_enum! {
    /// `dai::IMUSensor` (the members this crate names).
    ImuSensor {
        AccelerometerRaw = DAI_IMU_ACCELEROMETER_RAW,
        AccelerometerCalibrated = DAI_IMU_ACCELEROMETER_CALIBRATED,
        GyroscopeRaw = DAI_IMU_GYROSCOPE_RAW,
        MagnetometerRaw = DAI_IMU_MAGNETOMETER_RAW,
        RotationVector = DAI_IMU_ROTATION_VECTOR,
    }
}

dai_enum! {
    /// `dai::IMUReport::Accuracy`.
    ImuAccuracy {
        Unreliable = DAI_IMU_ACCURACY_UNRELIABLE,
        Low = DAI_IMU_ACCURACY_LOW,
        Medium = DAI_IMU_ACCURACY_MEDIUM,
        High = DAI_IMU_ACCURACY_HIGH,
    }
}

dai_enum! {
    /// `dai::VideoEncoderProperties::Profile`.
    VideoEncoderProfile {
        H264Baseline = DAI_VENC_H264_BASELINE,
        H264High = DAI_VENC_H264_HIGH,
        H264Main = DAI_VENC_H264_MAIN,
        H265Main = DAI_VENC_H265_MAIN,
        Mjpeg = DAI_VENC_MJPEG,
    }
}

dai_enum! {
    /// `dai::VideoEncoderProperties::RateControlMode`.
    RateControlMode {
        Cbr = DAI_VENC_RC_CBR,
        Vbr = DAI_VENC_RC_VBR,
    }
}

dai_enum! {
    /// `dai::node::StereoDepth::PresetMode`.
    StereoPresetMode {
        FastAccuracy = DAI_STEREO_PRESET_FAST_ACCURACY,
        FastDensity = DAI_STEREO_PRESET_FAST_DENSITY,
        Default = DAI_STEREO_PRESET_DEFAULT,
        Face = DAI_STEREO_PRESET_FACE,
        HighDetail = DAI_STEREO_PRESET_HIGH_DETAIL,
        Robotics = DAI_STEREO_PRESET_ROBOTICS,
        Density = DAI_STEREO_PRESET_DENSITY,
        Accuracy = DAI_STEREO_PRESET_ACCURACY,
    }
}

dai_enum! {
    /// `XLinkDeviceState_t` — what state an enumerated device is in.
    DeviceState {
        Any = DAI_XLINK_STATE_ANY,
        Booted = DAI_XLINK_STATE_BOOTED,
        Unbooted = DAI_XLINK_STATE_UNBOOTED,
        /// Sitting in the bootloader (a PoE device wedged here needs a
        /// [`DeviceBootloader`](crate::DeviceBootloader) open+drop to reboot).
        Bootloader = DAI_XLINK_STATE_BOOTLOADER,
        FlashBooted = DAI_XLINK_STATE_FLASH_BOOTED,
    }
}

dai_enum! {
    /// `dai::Platform`.
    Platform {
        Rvc2 = DAI_PLATFORM_RVC2,
        Rvc3 = DAI_PLATFORM_RVC3,
        Rvc4 = DAI_PLATFORM_RVC4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_passes_unknown_through() {
        assert_eq!(CameraBoardSocket::from_raw(1), CameraBoardSocket::CamB);
        assert_eq!(CameraBoardSocket::CamB.to_raw(), 1);
        assert_eq!(CameraBoardSocket::from_raw(-1), CameraBoardSocket::Auto);
        assert_eq!(
            CameraBoardSocket::from_raw(42),
            CameraBoardSocket::Other(42)
        );
        assert_eq!(CameraBoardSocket::Other(42).to_raw(), 42);
        assert_eq!(ImgFrameType::from_raw(30), ImgFrameType::Gray8);
        assert_eq!(Datatype::from_raw(28), Datatype::MessageGroup);
        assert_eq!(ImuSensor::GyroscopeRaw.to_raw(), 0x15);
        assert_eq!(DeviceState::from_raw(3), DeviceState::Bootloader);
    }
}
