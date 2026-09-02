//! [`NeuralNetwork`]: `dai::node::NeuralNetwork`, raw inference; output is
//! [`NnData`] tensors the host decodes.

use depthai_sys as sys;

use crate::error::{check, Result};
use crate::message::{AnyMessage, Message, NnData};
use crate::model_zoo::NNModelDescription;
use crate::nn_archive::NNArchive;
use crate::node::node_type;
use crate::node::Camera;
use crate::port::{Input, Output};

node_type!(
    /// A `dai::node::NeuralNetwork`: runs an NNArchive on the device and emits
    /// its raw output tensors as [`NnData`]. Decoding is the caller's.
    NeuralNetwork,
    dai_pipeline_create_neural_network
);

impl NeuralNetwork {
    /// `NeuralNetwork::build(camera, model, fps)`: fetch `model` from the zoo for
    /// this device's platform, request a matching output from `camera` and link
    /// it. `fps` `None` = depthai's default.
    pub fn build_camera(
        &self,
        camera: &Camera,
        model: &NNModelDescription,
        fps: Option<f32>,
    ) -> Result<()> {
        model.with_raw(|raw| {
            check(unsafe {
                sys::dai_neural_network_build_camera(
                    self.0.raw(),
                    camera.0.raw(),
                    raw,
                    fps.unwrap_or(0.0),
                )
            })
        })
    }

    /// `NeuralNetwork::build(input, archive)`: link `input` and load `archive`.
    pub fn build_output<M: Message>(&self, output: &Output<M>, archive: &NNArchive) -> Result<()> {
        check(unsafe {
            sys::dai_neural_network_build_output(self.0.raw(), output.raw(), archive.raw())
        })
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
