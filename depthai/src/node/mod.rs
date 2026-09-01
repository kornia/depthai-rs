//! Pipeline nodes. Each node type is a thin, `Clone`able handle onto the
//! C++ node owned by its [`Pipeline`]; the [`Node`] trait carries what every
//! node shares (identity, port introspection, untyped port lookup).

mod camera;
mod imu;
mod stereo_depth;
mod sync;
mod video_encoder;

pub use camera::Camera;
pub use imu::Imu;
pub use stereo_depth::{PostProcessing, StereoDepth};
pub use sync::Sync;
pub use video_encoder::VideoEncoder;

use std::ptr::NonNull;
use std::sync::Arc;

use depthai_sys as sys;

use crate::error::{
    check, check_poll, cstring, static_string, take_native_error, take_string, Result,
};
use crate::message::{AnyMessage, Message};
use crate::pipeline::Pipeline;
use crate::port::{Input, Output};

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
#[derive(Clone)]
pub struct NodeHandle {
    inner: Arc<NodeInner>,
}

impl std::fmt::Debug for NodeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeHandle")
            .field("type", &self.type_name())
            .field("id", &self.id().ok())
            .finish()
    }
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
        let mut v = 0;
        check(unsafe { sys::dai_node_id(self.raw(), &mut v) })?;
        Ok(v)
    }

    /// The C++ node type name (e.g. `"Camera"`).
    pub fn type_name(&self) -> String {
        let mut p: *const std::os::raw::c_char = std::ptr::null();
        if unsafe { sys::dai_node_type_name(self.raw(), &mut p) } < 0 {
            let _ = take_native_error();
            return String::new();
        }
        unsafe { static_string(p) }
    }

    /// Every output as `"group/name"`.
    pub fn output_names(&self) -> Result<Vec<String>> {
        let mut p = std::ptr::null_mut();
        check(unsafe { sys::dai_node_output_names(self.raw(), &mut p) })?;
        Ok(unsafe { take_string(p) }
            .lines()
            .map(str::to_owned)
            .collect())
    }

    /// Every input as `"group/name"`.
    pub fn input_names(&self) -> Result<Vec<String>> {
        let mut p = std::ptr::null_mut();
        check(unsafe { sys::dai_node_input_names(self.raw(), &mut p) })?;
        Ok(unsafe { take_string(p) }
            .lines()
            .map(str::to_owned)
            .collect())
    }

    /// Look up a fixed output port by name (any group). `Ok(None)` when absent.
    pub fn output_by_name(&self, name: &str) -> Result<Option<Output<AnyMessage>>> {
        self.output_typed(name)
    }

    /// Look up a fixed input port by name (any group). `Ok(None)` when absent.
    pub fn input_by_name(&self, name: &str) -> Result<Option<Input>> {
        let c = cstring(name)?;
        let mut out: *mut sys::dai_input = std::ptr::null_mut();
        let empty = c"";
        if !check_poll(unsafe {
            sys::dai_node_input(self.raw(), empty.as_ptr(), c.as_ptr(), &mut out)
        })? {
            return Ok(None);
        }
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        Ok(Some(unsafe { Input::from_raw(self.clone(), raw) }))
    }

    /// A key of an input map (get-or-create), e.g. Sync's `inputs["left"]`.
    pub fn input_map_get(&self, map_name: &str, key: &str) -> Result<Input> {
        let m = cstring(map_name)?;
        let k = cstring(key)?;
        let mut out: *mut sys::dai_input = std::ptr::null_mut();
        check(unsafe {
            sys::dai_node_input_map_get(self.raw(), m.as_ptr(), k.as_ptr(), &mut out)
        })?;
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        Ok(unsafe { Input::from_raw(self.clone(), raw) })
    }

    pub(crate) fn output_typed<M: Message>(&self, name: &str) -> Result<Option<Output<M>>> {
        let c = cstring(name)?;
        let mut out: *mut sys::dai_output = std::ptr::null_mut();
        let empty = c"";
        if !check_poll(unsafe {
            sys::dai_node_output(self.raw(), empty.as_ptr(), c.as_ptr(), &mut out)
        })? {
            return Ok(None);
        }
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        Ok(Some(unsafe { Output::from_raw(self.clone(), raw) }))
    }

    /// A fixed port that the node type guarantees exists.
    pub(crate) fn output_required<M: Message>(&self, name: &str) -> Result<Output<M>> {
        self.output_typed(name)?.ok_or_else(|| {
            crate::DepthaiError::Malformed(format!(
                "{} has no output named {name:?}",
                self.type_name()
            ))
        })
    }

    pub(crate) fn input_required(&self, name: &str) -> Result<Input> {
        self.input_by_name(name)?.ok_or_else(|| {
            crate::DepthaiError::Malformed(format!(
                "{} has no input named {name:?}",
                self.type_name()
            ))
        })
    }
}

/// Shared surface of every typed node.
pub trait Node {
    fn handle(&self) -> &NodeHandle;

    fn id(&self) -> Result<i64> {
        self.handle().id()
    }
    fn type_name(&self) -> String {
        self.handle().type_name()
    }
    fn output_names(&self) -> Result<Vec<String>> {
        self.handle().output_names()
    }
    fn input_names(&self) -> Result<Vec<String>> {
        self.handle().input_names()
    }
    fn output_by_name(&self, name: &str) -> Result<Option<Output<AnyMessage>>> {
        self.handle().output_by_name(name)
    }
    fn input_by_name(&self, name: &str) -> Result<Option<Input>> {
        self.handle().input_by_name(name)
    }
    fn pipeline(&self) -> &Pipeline {
        self.handle().pipeline()
    }
}

/// A node type that [`Pipeline::create`] can construct.
pub trait NodeType: Node + Sized {
    fn create(pipeline: &Pipeline) -> Result<Self>;
}

/// Implement `Node` + `NodeType` for a newtype over `NodeHandle` created by the
/// given shim function.
macro_rules! node_type {
    ($name:ident, $create:ident) => {
        impl crate::node::Node for $name {
            fn handle(&self) -> &crate::node::NodeHandle {
                &self.0
            }
        }
        impl crate::node::NodeType for $name {
            fn create(pipeline: &crate::pipeline::Pipeline) -> crate::error::Result<Self> {
                let mut raw: *mut depthai_sys::dai_node = std::ptr::null_mut();
                crate::error::check(unsafe { depthai_sys::$create(pipeline.raw(), &mut raw) })?;
                let raw =
                    std::ptr::NonNull::new(raw).ok_or_else(crate::error::take_native_error)?;
                // SAFETY: fresh owned handle from the shim.
                Ok($name(unsafe {
                    crate::node::NodeHandle::from_raw(raw, pipeline.clone())
                }))
            }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }
    };
}
pub(crate) use node_type;
