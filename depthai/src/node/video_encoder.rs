//! [`VideoEncoder`]: `dai::node::VideoEncoder`, the on-device H.264/H.265/MJPEG
//! encoder.

use depthai_sys as sys;

use crate::enums::{RateControlMode, VideoEncoderProfile};
use crate::error::{check, Result};
use crate::message::{EncodedFrame, ImgFrame};
use crate::node::{node_type, NodeHandle};
use crate::port::{Input, Output};

/// A `dai::node::VideoEncoder`. Feed it NV12 (or YUV420p) frames.
#[derive(Clone)]
pub struct VideoEncoder(pub(crate) NodeHandle);
node_type!(VideoEncoder, dai_pipeline_create_video_encoder);

impl VideoEncoder {
    /// The `in` port.
    pub fn input(&self) -> Result<Input> {
        self.0.input_required("in")
    }

    /// Encoded bytes as an [`ImgFrame`] of type `Bitstream` (Annex-B NAL units).
    pub fn bitstream(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("bitstream")
    }

    /// Encoded bytes as a typed [`EncodedFrame`] (frame type, profile, ...).
    pub fn out(&self) -> Result<Output<EncodedFrame>> {
        self.0.output_required("out")
    }

    pub fn set_default_profile_preset(&self, fps: f32, profile: VideoEncoderProfile) -> Result<()> {
        check(unsafe {
            sys::dai_video_encoder_set_default_profile_preset(self.0.raw(), fps, profile.to_raw())
        })
    }
    /// Keyframe every `freq` frames.
    pub fn set_keyframe_frequency(&self, freq: i32) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_keyframe_frequency(self.0.raw(), freq) })
    }
    pub fn set_bitrate_kbps(&self, kbps: i32) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_bitrate_kbps(self.0.raw(), kbps) })
    }
    pub fn set_bitrate(&self, bps: i32) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_bitrate(self.0.raw(), bps) })
    }
    pub fn set_profile(&self, profile: VideoEncoderProfile) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_profile(self.0.raw(), profile.to_raw()) })
    }
    pub fn set_rate_control_mode(&self, mode: RateControlMode) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_rate_control_mode(self.0.raw(), mode.to_raw()) })
    }
    pub fn set_num_bframes(&self, n: i32) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_num_bframes(self.0.raw(), n) })
    }
    /// MJPEG quality, 0..=100.
    pub fn set_quality(&self, quality: i32) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_quality(self.0.raw(), quality) })
    }
    pub fn set_lossless(&self, lossless: bool) -> Result<()> {
        check(unsafe { sys::dai_video_encoder_set_lossless(self.0.raw(), lossless as i32) })
    }
}
