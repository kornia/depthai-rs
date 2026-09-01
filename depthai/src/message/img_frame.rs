//! [`ImgFrame`]: one image (or encoder bitstream chunk) from the device.

use std::time::Duration;

use depthai_sys as sys;

use crate::enums::{Datatype, ImgFrameType};
use crate::error::{check, out_val, Result};
use crate::message::{Message, Msg, Sealed};

/// The metadata of an [`ImgFrame`], fetched in one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImgFrameInfo {
    pub width: u32,
    pub height: u32,
    /// Row stride in bytes as depthai reports it. **May be 0** on some paths;
    /// consumers that need a stride should treat 0 as "tightly packed" only after
    /// checking `data_len >= width * height * bytes_per_pixel`.
    pub stride: u32,
    pub img_type: ImgFrameType,
    /// `getInstanceNum()`: for camera frames, the board socket the frame came from.
    pub instance_num: u32,
    pub sequence_num: i64,
    /// Host `steady_clock` capture time, ns.
    pub timestamp_ns: i64,
    /// Device clock capture time, ns since boot.
    pub timestamp_device_ns: i64,
    /// `getData().size()`.
    pub data_len: usize,
}

/// A `dai::ImgFrame`. Cloning shares the buffer; [`data`](Self::data) is
/// zero-copy and valid as long as any clone lives.
#[derive(Clone)]
pub struct ImgFrame {
    msg: Msg,
    info: ImgFrameInfo,
    data: *const u8,
}

// SAFETY: `data` points into the message buffer owned (refcounted) by `msg`,
// which is itself Send + Sync; the bytes are never mutated.
unsafe impl Send for ImgFrame {}
unsafe impl Sync for ImgFrame {}

impl std::fmt::Debug for ImgFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImgFrame")
            .field("info", &self.info)
            .finish()
    }
}

impl Sealed for ImgFrame {}
impl Message for ImgFrame {
    const DATATYPE: Option<Datatype> = Some(Datatype::ImgFrame);

    unsafe fn from_msg(msg: Msg) -> Result<Self> {
        let mut raw = sys::dai_img_frame_info::default();
        // SAFETY: `msg` is a live ImgFrame handle; out-params are valid.
        check(unsafe { sys::dai_img_frame_get_info(msg.raw(), &mut raw) })?;
        let (data, data_len) = msg.data_raw()?;
        let info = ImgFrameInfo {
            width: raw.width,
            height: raw.height,
            stride: raw.stride,
            img_type: ImgFrameType::from_raw(raw.type_),
            instance_num: raw.instance_num,
            sequence_num: raw.sequence_num,
            timestamp_ns: raw.timestamp_ns,
            timestamp_device_ns: raw.timestamp_device_ns,
            data_len,
        };
        Ok(ImgFrame { msg, info, data })
    }

    fn as_msg(&self) -> &Msg {
        &self.msg
    }

    fn timestamp(&self) -> Duration {
        Duration::from_nanos(self.info.timestamp_ns.max(0) as u64)
    }

    fn timestamp_device(&self) -> Duration {
        Duration::from_nanos(self.info.timestamp_device_ns.max(0) as u64)
    }

    fn sequence_num(&self) -> i64 {
        self.info.sequence_num
    }
}

impl ImgFrame {
    /// All metadata at once.
    pub fn info(&self) -> &ImgFrameInfo {
        &self.info
    }

    /// The pixel (or bitstream) bytes, zero-copy. Layout depends on
    /// [`img_type`](Self::img_type) and [`stride`](Self::stride).
    pub fn data(&self) -> &[u8] {
        if self.data.is_null() || self.info.data_len == 0 {
            return &[];
        }
        // SAFETY: `data`/`data_len` came from the shim for the buffer `msg` keeps
        // alive; the slice borrows `self`.
        unsafe { std::slice::from_raw_parts(self.data, self.info.data_len) }
    }

    pub fn width(&self) -> u32 {
        self.info.width
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

    /// Host `steady_clock` capture time in ns (raw, not converted).
    pub fn timestamp_ns(&self) -> i64 {
        self.info.timestamp_ns
    }

    /// `getPlaneStride(plane)` for planar formats.
    pub fn plane_stride(&self, plane: i32) -> Result<u32> {
        out_val(|v| unsafe { sys::dai_img_frame_plane_stride(self.msg.raw(), plane, v) })
    }

    /// `getPlaneHeight()` for planar formats.
    pub fn plane_height(&self) -> Result<u32> {
        out_val(|v| unsafe { sys::dai_img_frame_plane_height(self.msg.raw(), v) })
    }

    /// The underlying untyped handle.
    pub fn as_msg(&self) -> &Msg {
        &self.msg
    }
}
