//! [`DetectionNetwork`]: `dai::node::DetectionNetwork`, a NeuralNetwork plus the
//! on-device parser that turns YOLO-style heads into [`ImgDetections`].

use depthai_sys as sys;

use crate::enums::ImgResizeMode;
use crate::error::{check, out_lines, Result};
use crate::message::{AnyMessage, ImgDetections, Message, NnData};
use crate::model_zoo::NNModelDescription;
use crate::nn_archive::NNArchive;
use crate::node::neural_network::{build_camera, build_output};
use crate::node::node_type;
use crate::node::Camera;
use crate::port::{Input, Output};

node_type!(
    /// A `dai::node::DetectionNetwork`: inference + on-device decoding for the
    /// detection heads depthai's `DetectionParser` knows (YOLO, SSD). Its ports are
    /// references into its two subnodes, reached through the accessors below.
    DetectionNetwork,
    dai_pipeline_create_detection_network
);

impl DetectionNetwork {
    /// `DetectionNetwork::build(camera, model, fps, resizeMode)`; `None` =
    /// depthai's defaults (unthrottled, `Crop`).
    pub fn build_camera(
        &self,
        camera: &Camera,
        model: &NNModelDescription,
        fps: Option<f32>,
        resize: Option<ImgResizeMode>,
    ) -> Result<()> {
        build_camera(
            &self.0,
            sys::dai_detection_network_build_camera,
            camera,
            model,
            fps,
            resize,
        )
    }

    /// `DetectionNetwork::build(output, archive)`: link `output` into `input` and load `archive`.
    pub fn build_output<M: Message>(&self, output: &Output<M>, archive: &NNArchive) -> Result<()> {
        build_output(
            &self.0,
            sys::dai_detection_network_build_output,
            output,
            archive,
        )
    }

    pub fn set_confidence_threshold(&self, threshold: f32) -> Result<()> {
        check(unsafe {
            sys::dai_detection_network_set_confidence_threshold(self.0.raw(), threshold)
        })
    }

    pub fn input(&self) -> Result<Input> {
        self.0.input_from(sys::dai_detection_network_input)
    }

    /// Decoded detections.
    pub fn out(&self) -> Result<Output<ImgDetections>> {
        self.0.output_from(sys::dai_detection_network_out)
    }

    /// The raw tensors the parser decoded.
    pub fn out_network(&self) -> Result<Output<NnData>> {
        self.0.output_from(sys::dai_detection_network_out_network)
    }

    pub fn passthrough(&self) -> Result<Output<AnyMessage>> {
        self.0.output_from(sys::dai_detection_network_passthrough)
    }

    /// `getClasses()`: label names from the archive, index = `ImgDetection::label`;
    /// empty when the archive carries none.
    pub fn classes(&self) -> Result<Vec<String>> {
        out_lines(|out| unsafe { sys::dai_detection_network_classes(self.0.raw(), out) })
    }
}
