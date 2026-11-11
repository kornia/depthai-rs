//! [`ImgFrame`]: one image (or encoder bitstream chunk) from the device.

use std::time::Duration;

use depthai_sys as sys;

use crate::enums::{Datatype, ImgFrameType};
use crate::error::{out_val, Result};
use crate::message::{AnyMessage, Message, Sealed};

/// The image-specific metadata of an [`ImgFrame`] (timestamps, sequence number
/// and byte length are in the [`MessageHeader`](crate::MessageHeader)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImgFrameInfo {
    pub width: u32,
    pub height: u32,
    /// Row stride in bytes as depthai reports it. **May be 0** on some paths;
    /// treat 0 as "tightly packed" only after checking
    /// `data().len() >= width * height * bytes_per_pixel`.
    pub stride: u32,
    pub img_type: ImgFrameType,
    /// `getInstanceNum()`: for camera frames, the board socket the frame came from.
    pub instance_num: u32,
}

/// A `dai::ImgFrame`. Cloning shares the buffer; [`data`](Message::data) is
/// zero-copy and valid as long as any clone lives.
#[derive(Clone, Debug)]
pub struct ImgFrame {
    any: AnyMessage,
    info: ImgFrameInfo,
    exposure: Duration,
}

impl Sealed for ImgFrame {}
impl Message for ImgFrame {
    const DATATYPE: Option<Datatype> = Some(Datatype::ImgFrame);

    fn from_any(any: AnyMessage) -> Result<Self> {
        let raw: sys::dai_img_frame_info =
            out_val(|out| unsafe { sys::dai_img_frame_get_info(any.raw(), out) })?;
        let info = ImgFrameInfo {
            width: raw.width,
            height: raw.height,
            stride: raw.stride,
            img_type: ImgFrameType::from_raw(raw.type_),
            instance_num: raw.instance_num,
        };
        Ok(ImgFrame {
            any,
            info,
            exposure: Duration::from_micros(raw.exposure_us),
        })
    }

    fn as_any(&self) -> &AnyMessage {
        &self.any
    }
}

impl ImgFrame {
    pub fn info(&self) -> &ImgFrameInfo {
        &self.info
    }

    /// The pixel (or bitstream) bytes, zero-copy. Layout depends on
    /// [`img_type`](Self::img_type) and [`stride`](Self::stride).
    pub fn data(&self) -> &[u8] {
        self.any.data()
    }

    pub fn width(&self) -> u32 {
        self.info.width
    }

    /// `getExposureTime()`: how long the sensor exposed this frame. [`Duration::ZERO`] when
    /// the frame carries none. [`timestamp`](Message::timestamp) is the END of that exposure,
    /// so `timestamp - exposure / 2` is `getTimestamp(CameraExposureOffset::MIDDLE)`.
    pub fn exposure(&self) -> Duration {
        self.exposure
    }

    pub fn height(&self) -> u32 {
        self.info.height
    }

    /// Row stride in bytes as depthai reports it (may be 0 — see [`ImgFrameInfo::stride`]).
    pub fn stride(&self) -> u32 {
        self.info.stride
    }

    pub fn img_type(&self) -> ImgFrameType {
        self.info.img_type
    }

    pub fn instance_num(&self) -> u32 {
        self.info.instance_num
    }

    /// `getPlaneStride(plane)` for planar formats.
    pub fn plane_stride(&self, plane: i32) -> Result<u32> {
        out_val(|v| unsafe { sys::dai_img_frame_plane_stride(self.any.raw(), plane, v) })
    }

    /// `getPlaneHeight()` for planar formats.
    pub fn plane_height(&self) -> Result<u32> {
        out_val(|v| unsafe { sys::dai_img_frame_plane_height(self.any.raw(), v) })
    }
}
