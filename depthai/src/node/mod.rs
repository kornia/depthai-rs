//! Pipeline nodes. Each node type is a thin, `Clone`able handle onto the
//! C++ node owned by its [`Pipeline`]; the [`Node`] trait carries what every
//! node shares (identity, port introspection, port lookup by name).

mod camera;
mod detection_network;
mod gate;
mod imu;
pub(crate) mod neural_network;
mod stereo_depth;
mod sync;
mod video_encoder;

pub use camera::Camera;
pub use detection_network::DetectionNetwork;
pub use gate::Gate;
pub use imu::Imu;
pub use neural_network::NeuralNetwork;
pub use stereo_depth::{PostProcessing, StereoDepth};
pub use sync::Sync;
pub use video_encoder::VideoEncoder;

use std::os::raw::c_int;
use std::ptr::NonNull;
use std::sync::Arc;

use depthai_sys as sys;

use crate::error::{
    check, cstring, out_handle, out_lines, out_val, poll_handle, static_string, DepthaiError,
    Result,
};
use crate::message::{AnyMessage, Message};
use crate::pipeline::Pipeline;
use crate::port::{Input, Output};

#[derive(Debug)]
pub(crate) struct NodeInner {
    raw: NonNull<sys::dai_node>,
    /// Keeps the pipeline (and through it the device) alive while any handle to
    /// the node, or any port borrowed from it, exists.
    pipeline: Pipeline,
}

// SAFETY: a heap shared_ptr copy (atomic control block). depthai-core's Node has
// no internal locking, so every shim entry point that reads or mutates a node,
// a port, or builds/starts the pipeline takes the shim's global
// `g_graph_mutex` (DAI_LOCK_GRAPH in depthai_c.cpp). That lock is what makes
// concurrent calls through this handle data-race free.
unsafe impl Send for NodeInner {}
unsafe impl std::marker::Sync for NodeInner {}

impl Drop for NodeInner {
    fn drop(&mut self) {
        unsafe { sys::dai_node_release(self.raw.as_ptr()) };
    }
}

/// An untyped handle to any node. The typed node structs wrap one of these.
#[derive(Clone, Debug)]
pub struct NodeHandle {
    inner: Arc<NodeInner>,
}

/// Split a `"group/name"` port spec (as [`Node::output_names`] emits) into its
/// parts; a bare name means the default group.
fn split_port(spec: &str) -> (&str, &str) {
    spec.split_once('/').unwrap_or(("", spec))
}

impl NodeHandle {
    /// # Safety
    /// `raw` must be a live handle from a `dai_pipeline_create_*` call.
    pub(crate) unsafe fn from_raw(raw: NonNull<sys::dai_node>, pipeline: Pipeline) -> Self {
        NodeHandle {
            inner: Arc::new(NodeInner { raw, pipeline }),
        }
    }

    pub(crate) fn raw(&self) -> *mut sys::dai_node {
        self.inner.raw.as_ptr()
    }

    pub fn pipeline(&self) -> &Pipeline {
        &self.inner.pipeline
    }

    /// The node id assigned by the pipeline.
    pub fn id(&self) -> Result<i64> {
        out_val(|v| unsafe { sys::dai_node_id(self.raw(), v) })
    }

    /// The C++ node type name (e.g. `"Camera"`).
    pub fn type_name(&self) -> Result<String> {
        let mut p: *const std::os::raw::c_char = std::ptr::null();
        check(unsafe { sys::dai_node_type_name(self.raw(), &mut p) })?;
        // SAFETY: the shim returns a string with static storage.
        Ok(unsafe { static_string(p) })
    }

    /// Every output as `"group/name"`.
    pub fn output_names(&self) -> Result<Vec<String>> {
        out_lines(|p| unsafe { sys::dai_node_output_names(self.raw(), p) })
    }

    /// Every input as `"group/name"`.
    pub fn input_names(&self) -> Result<Vec<String>> {
        out_lines(|p| unsafe { sys::dai_node_input_names(self.raw(), p) })
    }

    /// `Node::getOutputRef(group, name)`, typed as `M`. `spec` is `"name"` (default
    /// group) or `"group/name"` as [`output_names`](Self::output_names) emits.
    /// `Ok(None)` when absent.
    pub fn output<M: Message>(&self, spec: &str) -> Result<Option<Output<M>>> {
        let (group, name) = split_port(spec);
        let (g, n) = (cstring(group)?, cstring(name)?);
        let found = poll_handle(|out| unsafe {
            sys::dai_node_output_ref(self.raw(), g.as_ptr(), n.as_ptr(), out)
        })?;
        // SAFETY: the shim handed out a port owned by this node.
        Ok(found.map(|raw| unsafe { Output::from_raw(self.clone(), raw) }))
    }

    /// [`output`](Self::output) untyped.
    pub fn output_by_name(&self, spec: &str) -> Result<Option<Output<AnyMessage>>> {
        self.output(spec)
    }

    /// `Node::getInputRef(group, name)`; `spec` as for [`output`](Self::output).
    /// `Ok(None)` when absent.
    pub fn input_by_name(&self, spec: &str) -> Result<Option<Input>> {
        let (group, name) = split_port(spec);
        let (g, n) = (cstring(group)?, cstring(name)?);
        let found = poll_handle(|out| unsafe {
            sys::dai_node_input_ref(self.raw(), g.as_ptr(), n.as_ptr(), out)
        })?;
        // SAFETY: the shim handed out a port owned by this node.
        Ok(found.map(|raw| unsafe { Input::from_raw(self.clone(), raw) }))
    }

    /// A port reached through a dedicated shim accessor (a node whose ports are
    /// references into subnodes, invisible to the name lookup).
    pub(crate) fn output_from<M: Message>(
        &self,
        f: unsafe extern "C" fn(*mut sys::dai_node, *mut *mut sys::dai_output) -> c_int,
    ) -> Result<Output<M>> {
        let raw = out_handle(|out| unsafe { f(self.raw(), out) })?;
        // SAFETY: the shim handed out a port owned by this node.
        Ok(unsafe { Output::from_raw(self.clone(), raw) })
    }

    pub(crate) fn input_from(
        &self,
        f: unsafe extern "C" fn(*mut sys::dai_node, *mut *mut sys::dai_input) -> c_int,
    ) -> Result<Input> {
        let raw = out_handle(|out| unsafe { f(self.raw(), out) })?;
        // SAFETY: as in `output_from`.
        Ok(unsafe { Input::from_raw(self.clone(), raw) })
    }

    /// A fixed port that the node type guarantees exists.
    pub(crate) fn output_required<M: Message>(&self, name: &str) -> Result<Output<M>> {
        self.output(name)?
            .ok_or_else(|| self.missing_port("output", name))
    }

    pub(crate) fn input_required(&self, name: &str) -> Result<Input> {
        self.input_by_name(name)?
            .ok_or_else(|| self.missing_port("input", name))
    }

    fn missing_port(&self, kind: &'static str, name: &str) -> DepthaiError {
        DepthaiError::MissingPort {
            node: self.type_name().unwrap_or_default(),
            kind,
            name: name.to_owned(),
        }
    }
}

/// Shared surface of every typed node.
pub trait Node {
    fn handle(&self) -> &NodeHandle;

    fn id(&self) -> Result<i64> {
        self.handle().id()
    }
    fn type_name(&self) -> Result<String> {
        self.handle().type_name()
    }
    fn output_names(&self) -> Result<Vec<String>> {
        self.handle().output_names()
    }
    fn input_names(&self) -> Result<Vec<String>> {
        self.handle().input_names()
    }
    fn output_by_name(&self, spec: &str) -> Result<Option<Output<AnyMessage>>> {
        self.handle().output_by_name(spec)
    }
    fn input_by_name(&self, spec: &str) -> Result<Option<Input>> {
        self.handle().input_by_name(spec)
    }
    fn pipeline(&self) -> &Pipeline {
        self.handle().pipeline()
    }
}

impl Node for NodeHandle {
    fn handle(&self) -> &NodeHandle {
        self
    }
}

/// A node type that [`Pipeline::create`] can construct.
pub trait NodeType: Node + Sized {
    fn create(pipeline: &Pipeline) -> Result<Self>;
}

/// Declare a typed node: a newtype over `NodeHandle` created by the given shim
/// function, with `Node` + `NodeType` implemented.
macro_rules! node_type {
    ($(#[$meta:meta])* $name:ident, $create:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug)]
        pub struct $name(pub(crate) crate::node::NodeHandle);

        impl crate::node::Node for $name {
            fn handle(&self) -> &crate::node::NodeHandle {
                &self.0
            }
        }
        impl crate::node::NodeType for $name {
            fn create(pipeline: &crate::pipeline::Pipeline) -> crate::error::Result<Self> {
                let raw = crate::error::out_handle(|out| unsafe {
                    depthai_sys::$create(pipeline.raw(), out)
                })?;
                // SAFETY: fresh owned handle from the shim.
                Ok($name(unsafe {
                    crate::node::NodeHandle::from_raw(raw, pipeline.clone())
                }))
            }
        }
    };
}
pub(crate) use node_type;

#[cfg(test)]
mod tests {
    #[test]
    fn port_spec_splits_group() {
        assert_eq!(super::split_port("depth"), ("", "depth"));
        assert_eq!(super::split_port("inputs/left"), ("inputs", "left"));
    }
}
