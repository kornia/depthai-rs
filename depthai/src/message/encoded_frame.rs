//! [`EncodedFrame`]: the [`VideoEncoder`](crate::node::VideoEncoder)'s typed
//! output (`out`), as opposed to its `bitstream` [`ImgFrame`](super::ImgFrame).

use depthai_sys as sys;

use crate::enums::{Datatype, EncodedFrameProfile, EncodedFrameType};
use crate::error::{out_val, Result};
use crate::message::{AnyMessage, Message, Sealed};

/// The encoder-specific metadata of an [`EncodedFrame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodedFrameInfo {
    pub width: u32,
    pub height: u32,
    pub profile: EncodedFrameProfile,
    pub frame_type: EncodedFrameType,
    pub quality: u32,
    pub bitrate: u32,
    pub lossless: bool,
    pub instance_num: u32,
}

/// A `dai::EncodedFrame`.
#[derive(Clone, Debug)]
pub struct EncodedFrame {
    any: AnyMessage,
    info: EncodedFrameInfo,
}

impl Sealed for EncodedFrame {}
impl Message for EncodedFrame {
    const DATATYPE: Option<Datatype> = Some(Datatype::EncodedFrame);

    fn from_any(any: AnyMessage) -> Result<Self> {
        let raw: sys::dai_encoded_frame_info =
            out_val(|out| unsafe { sys::dai_encoded_frame_get_info(any.raw(), out) })?;
        let info = EncodedFrameInfo {
            width: raw.width,
            height: raw.height,
            profile: EncodedFrameProfile::from_raw(raw.profile),
            frame_type: EncodedFrameType::from_raw(raw.frame_type),
            quality: raw.quality,
            bitrate: raw.bitrate,
            lossless: raw.lossless != 0,
            instance_num: raw.instance_num,
        };
        Ok(EncodedFrame { any, info })
    }

    fn as_any(&self) -> &AnyMessage {
        &self.any
    }
}

impl EncodedFrame {
    pub fn info(&self) -> &EncodedFrameInfo {
        &self.info
    }

    /// The encoded bytes (Annex-B NAL units for H.264/H.265), zero-copy.
    pub fn data(&self) -> &[u8] {
        self.any.data()
    }

    pub fn is_keyframe(&self) -> bool {
        self.info.frame_type == EncodedFrameType::I
    }
}
