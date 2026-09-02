//! [`NeuralNetwork`]: `dai::node::NeuralNetwork`, raw inference; output is
//! [`NnData`] tensors the host decodes.

use std::os::raw::c_int;

use depthai_sys as sys;

use crate::enums::{opt_raw, ImgResizeMode};
use crate::error::{check, Result};
use crate::message::{AnyMessage, Message, NnData};
use crate::model_zoo::NNModelDescription;
use crate::nn_archive::NNArchive;
use crate::node::node_type;
use crate::node::{Camera, NodeHandle};
use crate::port::{Input, Output};

node_type!(
    /// A `dai::node::NeuralNetwork`: runs an NNArchive on the device and emits
    /// its raw output tensors as [`NnData`]. Decoding is the caller's.
    NeuralNetwork,
    dai_pipeline_create_neural_network
);

/// The `build(camera, model, fps, resizeMode)` C mirror shared with `DetectionNetwork`.
pub(crate) type BuildCameraFn = unsafe extern "C" fn(
    *mut sys::dai_node,
    *mut sys::dai_node,
    *const sys::dai_nn_model_description,
    f32,
    i32,
) -> c_int;
/// The `build(output, archive)` C mirror shared with `DetectionNetwork`.
pub(crate) type BuildOutputFn = unsafe extern "C" fn(
    *mut sys::dai_node,
    *mut sys::dai_output,
    *const sys::dai_nn_archive,
) -> c_int;

pub(crate) fn build_camera(
    node: &NodeHandle,
    f: BuildCameraFn,
    camera: &Camera,
    model: &NNModelDescription,
    fps: Option<f32>,
    resize: Option<ImgResizeMode>,
) -> Result<()> {
    model.with_raw(|raw| {
        check(unsafe {
            f(
                node.raw(),
                camera.0.raw(),
                raw,
                fps.unwrap_or(0.0),
                opt_raw(resize),
            )
        })
    })
}

pub(crate) fn build_output<M: Message>(
    node: &NodeHandle,
    f: BuildOutputFn,
    output: &Output<M>,
    archive: &NNArchive,
) -> Result<()> {
    check(unsafe { f(node.raw(), output.raw(), archive.raw()) })
}

impl NeuralNetwork {
    /// `NeuralNetwork::build(camera, model, fps, resizeMode)`: fetch `model` from
    /// the zoo for this device's platform, request a matching output from
    /// `camera` and link it. `None` = depthai's defaults (unthrottled, `Crop`).
    pub fn build_camera(
        &self,
        camera: &Camera,
        model: &NNModelDescription,
        fps: Option<f32>,
        resize: Option<ImgResizeMode>,
    ) -> Result<()> {
        build_camera(
            &self.0,
            sys::dai_neural_network_build_camera,
            camera,
            model,
            fps,
            resize,
        )
    }

    /// `NeuralNetwork::build(output, archive)`: link `output` into `in` and load `archive`.
    pub fn build_output<M: Message>(&self, output: &Output<M>, archive: &NNArchive) -> Result<()> {
        build_output(
            &self.0,
            sys::dai_neural_network_build_output,
            output,
            archive,
        )
    }

    pub fn set_nn_archive(&self, archive: &NNArchive) -> Result<()> {
        check(unsafe { sys::dai_neural_network_set_nn_archive(self.0.raw(), archive.raw()) })
    }

    pub fn set_num_inference_threads(&self, n: i32) -> Result<()> {
        check(unsafe { sys::dai_neural_network_set_num_inference_threads(self.0.raw(), n) })
    }

    pub fn set_num_pool_frames(&self, n: i32) -> Result<()> {
        check(unsafe { sys::dai_neural_network_set_num_pool_frames(self.0.raw(), n) })
    }

    pub fn input(&self) -> Result<Input> {
        self.0.input_required("in")
    }

    /// Raw output tensors.
    pub fn out(&self) -> Result<Output<NnData>> {
        self.0.output_required("out")
    }

    /// The input frame each inference ran on.
    pub fn passthrough(&self) -> Result<Output<AnyMessage>> {
        self.0.output_required("passthrough")
    }
}
