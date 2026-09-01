//! [`EncodedFrame`]: the [`VideoEncoder`](crate::node::VideoEncoder)'s typed
//! output (`out`), as opposed to its `bitstream` [`ImgFrame`](super::ImgFrame).

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{check, Result};
use crate::message::{Message, Msg, Sealed};

/// Metadata of an [`EncodedFrame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodedFrameInfo {
    pub width: u32,
    pub height: u32,
    /// `dai::EncodedFrame::Profile`: 0 JPEG, 1 AVC (H.264), 2 HEVC (H.265).
    pub profile: i32,
    /// `dai::EncodedFrame::FrameType`: 0 I, 1 P, 2 B, 3 Unknown.
    pub frame_type: i32,
    pub quality: u32,
    pub bitrate: u32,
    pub lossless: bool,
    pub instance_num: u32,
    pub sequence_num: i64,
    pub timestamp_ns: i64,
    pub data_len: usize,
}

/// A `dai::EncodedFrame`.
#[derive(Clone)]
pub struct EncodedFrame {
    msg: Msg,
    info: EncodedFrameInfo,
    data: *const u8,
}

// SAFETY: as for ImgFrame — `data` is owned by the refcounted, immutable `msg`.
unsafe impl Send for EncodedFrame {}
unsafe impl Sync for EncodedFrame {}

impl std::fmt::Debug for EncodedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncodedFrame")
            .field("info", &self.info)
            .finish()
    }
}

impl Sealed for EncodedFrame {}
impl Message for EncodedFrame {
    const DATATYPE: Option<Datatype> = Some(Datatype::EncodedFrame);

    unsafe fn from_msg(msg: Msg) -> Result<Self> {
        let mut raw = sys::dai_encoded_frame_info::default();
        // SAFETY: `msg` is a live EncodedFrame handle; out-params are valid.
        check(unsafe { sys::dai_encoded_frame_get_info(msg.raw(), &mut raw) })?;
        let mut p: *const u8 = std::ptr::null();
        let mut len: usize = 0;
        check(unsafe { sys::dai_msg_data(msg.raw(), &mut p, &mut len) })?;
        let info = EncodedFrameInfo {
            width: raw.width,
            height: raw.height,
            profile: raw.profile,
            frame_type: raw.frame_type,
            quality: raw.quality,
            bitrate: raw.bitrate,
            lossless: raw.lossless != 0,
            instance_num: raw.instance_num,
            sequence_num: raw.sequence_num,
            timestamp_ns: raw.timestamp_ns,
            data_len: len,
        };
        Ok(EncodedFrame { msg, info, data: p })
    }

    fn as_msg(&self) -> &Msg {
        &self.msg
    }

    fn sequence_num(&self) -> i64 {
        self.info.sequence_num
    }
}

impl EncodedFrame {
    pub fn info(&self) -> &EncodedFrameInfo {
        &self.info
    }

    /// The encoded bytes (Annex-B NAL units for H.264/H.265), zero-copy.
    pub fn data(&self) -> &[u8] {
        if self.data.is_null() || self.info.data_len == 0 {
            return &[];
        }
        // SAFETY: see ImgFrame::data.
        unsafe { std::slice::from_raw_parts(self.data, self.info.data_len) }
    }

    /// True for an I-frame (keyframe).
    pub fn is_keyframe(&self) -> bool {
        self.info.frame_type == 0
    }
}
