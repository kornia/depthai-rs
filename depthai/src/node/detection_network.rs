//! [`DetectionNetwork`]: `dai::node::DetectionNetwork`, a NeuralNetwork plus the
//! on-device parser that turns YOLO-style heads into [`ImgDetections`].

use depthai_sys as sys;

use crate::error::{check, out_handle, out_lines, Result};
use crate::message::{AnyMessage, ImgDetections, Message, NnData};
use crate::model_zoo::NNModelDescription;
use crate::nn_archive::NNArchive;
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
    /// `DetectionNetwork::build(camera, model, fps)`.
    pub fn build_camera(
        &self,
        camera: &Camera,
        model: &NNModelDescription,
        fps: Option<f32>,
    ) -> Result<()> {
        model.with_raw(|raw| {
            check(unsafe {
                sys::dai_detection_network_build_camera(
                    self.0.raw(),
                    camera.0.raw(),
                    raw,
                    fps.unwrap_or(0.0),
                )
            })
        })
    }

    /// `DetectionNetwork::build(input, archive)`.
    pub fn build_output<M: Message>(&self, input: &Output<M>, archive: &NNArchive) -> Result<()> {
        check(unsafe {
            sys::dai_detection_network_build_output(self.0.raw(), input.raw(), archive.raw())
        })
    }

    pub fn set_confidence_threshold(&self, threshold: f32) -> Result<()> {
        check(unsafe {
            sys::dai_detection_network_set_confidence_threshold(self.0.raw(), threshold)
        })
    }

    pub fn input(&self) -> Result<Input> {
        let raw = out_handle(|out| unsafe { sys::dai_detection_network_input(self.0.raw(), out) })?;
        // SAFETY: a port owned by this node's subnode; the node handle keeps it alive.
        Ok(unsafe { Input::from_raw(self.0.clone(), raw) })
    }

    /// Decoded detections.
    pub fn out(&self) -> Result<Output<ImgDetections>> {
        let raw = out_handle(|out| unsafe { sys::dai_detection_network_out(self.0.raw(), out) })?;
        // SAFETY: as in `input`.
        Ok(unsafe { Output::from_raw(self.0.clone(), raw) })
    }

    /// The raw tensors the parser decoded.
    pub fn out_network(&self) -> Result<Output<NnData>> {
        let raw =
            out_handle(|out| unsafe { sys::dai_detection_network_out_network(self.0.raw(), out) })?;
        // SAFETY: as in `input`.
        Ok(unsafe { Output::from_raw(self.0.clone(), raw) })
    }

    pub fn passthrough(&self) -> Result<Output<AnyMessage>> {
        let raw =
            out_handle(|out| unsafe { sys::dai_detection_network_passthrough(self.0.raw(), out) })?;
        // SAFETY: as in `input`.
        Ok(unsafe { Output::from_raw(self.0.clone(), raw) })
    }

    /// `getClasses()`: label names from the archive, index = `ImgDetection::label`;
    /// empty when the archive carries none.
    pub fn classes(&self) -> Result<Vec<String>> {
        out_lines(|out| unsafe { sys::dai_detection_network_classes(self.0.raw(), out) })
    }
}
