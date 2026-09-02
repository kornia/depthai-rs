//! [`StereoDepth`]: `dai::node::StereoDepth`, on-device stereo matching.

use depthai_sys as sys;

use crate::enums::{CameraBoardSocket, StereoPresetMode};
use crate::error::{check, Result};
use crate::message::ImgFrame;
use crate::node::node_type;
use crate::port::{Input, Output};

node_type!(
    /// A `dai::node::StereoDepth`.
    StereoDepth,
    dai_pipeline_create_stereo_depth
);

impl StereoDepth {
    pub fn left(&self) -> Result<Input> {
        self.0.input_required("left")
    }
    pub fn right(&self) -> Result<Input> {
        self.0.input_required("right")
    }
    /// Feed a stream here to align the depth output to *that output's* grid
    /// (same crop/size/intrinsics), not just to its socket.
    pub fn input_align_to(&self) -> Result<Input> {
        self.0.input_required("inputAlignTo")
    }

    /// Depth in millimetres as `RAW16` (0 = no return).
    pub fn depth(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("depth")
    }
    pub fn disparity(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("disparity")
    }
    pub fn rectified_left(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("rectifiedLeft")
    }
    pub fn rectified_right(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("rectifiedRight")
    }
    pub fn synced_left(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("syncedLeft")
    }
    pub fn synced_right(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("syncedRight")
    }
    pub fn confidence_map(&self) -> Result<Output<ImgFrame>> {
        self.0.output_required("confidenceMap")
    }

    pub fn set_default_profile_preset(&self, preset: StereoPresetMode) -> Result<()> {
        check(unsafe {
            sys::dai_stereo_depth_set_default_profile_preset(self.0.raw(), preset.to_raw())
        })
    }
    pub fn set_left_right_check(&self, enable: bool) -> Result<()> {
        check(unsafe { sys::dai_stereo_depth_set_left_right_check(self.0.raw(), enable as i32) })
    }
    pub fn set_subpixel(&self, enable: bool) -> Result<()> {
        check(unsafe { sys::dai_stereo_depth_set_subpixel(self.0.raw(), enable as i32) })
    }
    pub fn set_extended_disparity(&self, enable: bool) -> Result<()> {
        check(unsafe { sys::dai_stereo_depth_set_extended_disparity(self.0.raw(), enable as i32) })
    }
    /// Output size of the depth map. XLink requires even dimensions. Not supported
    /// on RVC4 (depthai throws at the call).
    pub fn set_output_size(&self, width: u32, height: u32) -> Result<()> {
        check(unsafe {
            sys::dai_stereo_depth_set_output_size(self.0.raw(), width as i32, height as i32)
        })
    }
    pub fn set_depth_align(&self, socket: CameraBoardSocket) -> Result<()> {
        check(unsafe {
            sys::dai_stereo_depth_set_depth_align_socket(self.0.raw(), socket.to_raw())
        })
    }
    pub fn set_confidence_threshold(&self, threshold: i32) -> Result<()> {
        check(unsafe { sys::dai_stereo_depth_set_confidence_threshold(self.0.raw(), threshold) })
    }

    /// `initialConfig->postProcessing` setters.
    pub fn post_processing(&self) -> PostProcessing<'_> {
        PostProcessing { node: self }
    }
}

/// Setters on `StereoDepth::initialConfig->postProcessing`.
#[derive(Debug)]
pub struct PostProcessing<'a> {
    node: &'a StereoDepth,
}

impl PostProcessing<'_> {
    pub fn set_spatial_filter_enable(&self, enable: bool) -> Result<&Self> {
        check(unsafe {
            sys::dai_stereo_depth_pp_set_spatial_filter_enable(self.node.0.raw(), enable as i32)
        })?;
        Ok(self)
    }
    pub fn set_temporal_filter_enable(&self, enable: bool) -> Result<&Self> {
        check(unsafe {
            sys::dai_stereo_depth_pp_set_temporal_filter_enable(self.node.0.raw(), enable as i32)
        })?;
        Ok(self)
    }
    pub fn set_speckle_filter_enable(&self, enable: bool) -> Result<&Self> {
        check(unsafe {
            sys::dai_stereo_depth_pp_set_speckle_filter_enable(self.node.0.raw(), enable as i32)
        })?;
        Ok(self)
    }
    /// Keep only depth within `[min_range, max_range]` millimetres.
    pub fn set_threshold_filter(&self, min_range_mm: i32, max_range_mm: i32) -> Result<&Self> {
        check(unsafe {
            sys::dai_stereo_depth_pp_set_threshold_filter(
                self.node.0.raw(),
                min_range_mm,
                max_range_mm,
            )
        })?;
        Ok(self)
    }
    pub fn set_decimation_factor(&self, factor: u32) -> Result<&Self> {
        check(unsafe {
            sys::dai_stereo_depth_pp_set_decimation_factor(self.node.0.raw(), factor)
        })?;
        Ok(self)
    }
}
