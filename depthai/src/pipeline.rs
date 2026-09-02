//! [`Pipeline`]: the node graph that runs on a device.

use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use depthai_sys as sys;

use crate::device::Device;
use crate::error::{check, out_bool, out_handle, Result};
use crate::node::NodeType;

#[derive(Debug)]
pub(crate) struct PipelineInner {
    raw: NonNull<sys::dai_pipeline>,
    device: Option<Device>,
    /// `PipelineImpl::wait()` joins node threads without any lock of its own;
    /// serialise concurrent `wait()` calls from clones here.
    wait_lock: Mutex<()>,
}

// SAFETY: PipelineImpl serialises its state internally; the handle is unique on
// the C++ side and shared here through the Arc.
unsafe impl Send for PipelineInner {}
unsafe impl Sync for PipelineInner {}

impl Drop for PipelineInner {
    fn drop(&mut self) {
        // The shim stops a running pipeline before destroying it.
        unsafe { sys::dai_pipeline_release(self.raw.as_ptr()) };
    }
}

/// A `dai::Pipeline`. Cloning shares the same pipeline; nodes, ports and queues
/// created from it keep it alive, and it keeps its [`Device`] alive.
///
/// Build the graph (create nodes, request outputs, link, create queues), then
/// [`start`](Self::start). Configure from one thread before starting.
///
/// Dropping the last handle stops the pipeline, which — as in depthai itself —
/// **closes its device and every output queue**: a [`Device`] clone kept past
/// that point is closed, and queues return `Err` from then on.
#[derive(Clone, Debug)]
pub struct Pipeline {
    inner: Arc<PipelineInner>,
}

impl Pipeline {
    /// A pipeline bound to `device` (`dai::Pipeline(std::shared_ptr<Device>)`).
    pub fn new(device: &Device) -> Result<Pipeline> {
        let raw = out_handle(|out| unsafe { sys::dai_pipeline_new(device.raw(), out) })?;
        Ok(Pipeline {
            inner: Arc::new(PipelineInner {
                raw,
                device: Some(device.clone()),
                wait_lock: Mutex::new(()),
            }),
        })
    }

    /// A pipeline with no device (`dai::Pipeline(false)`): build and inspect a
    /// graph without hardware. Cannot start.
    pub fn host_only() -> Result<Pipeline> {
        let raw = out_handle(|out| unsafe { sys::dai_pipeline_new_host_only(out) })?;
        Ok(Pipeline {
            inner: Arc::new(PipelineInner {
                raw,
                device: None,
                wait_lock: Mutex::new(()),
            }),
        })
    }

    pub(crate) fn raw(&self) -> *mut sys::dai_pipeline {
        self.inner.raw.as_ptr()
    }

    /// The device this pipeline runs on (`None` for [`host_only`](Self::host_only)).
    pub fn device(&self) -> Option<&Device> {
        self.inner.device.as_ref()
    }

    /// Create a node: `pipeline.create::<Camera>()?`.
    pub fn create<N: NodeType>(&self) -> Result<N> {
        N::create(self)
    }

    /// `Pipeline::remove(node)`: detach a node and its links from the (unstarted)
    /// graph — e.g. to undo a partially built optional subgraph.
    pub fn remove(&self, node: &impl crate::node::Node) -> Result<()> {
        check(unsafe { sys::dai_pipeline_remove(self.raw(), node.handle().raw()) })
    }

    pub fn build(&self) -> Result<()> {
        check(unsafe { sys::dai_pipeline_build(self.raw()) })
    }

    /// Build (if needed), upload and start the pipeline on the device.
    pub fn start(&self) -> Result<()> {
        check(unsafe { sys::dai_pipeline_start(self.raw()) })
    }

    /// Stop the pipeline: closes every output queue and the device.
    pub fn stop(&self) -> Result<()> {
        check(unsafe { sys::dai_pipeline_stop(self.raw()) })
    }

    /// Block until the pipeline stops (joins the node threads). Concurrent `wait`
    /// calls from clones are serialised; do not mutate the graph while waiting.
    pub fn wait(&self) -> Result<()> {
        let _guard = self
            .inner
            .wait_lock
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        check(unsafe { sys::dai_pipeline_wait(self.raw()) })
    }

    pub fn is_running(&self) -> Result<bool> {
        out_bool(|v| unsafe { sys::dai_pipeline_is_running(self.raw(), v) })
    }

    pub fn is_built(&self) -> Result<bool> {
        out_bool(|v| unsafe { sys::dai_pipeline_is_built(self.raw(), v) })
    }
}
