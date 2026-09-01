//! [`Output`] and [`Input`]: a node's ports. Link an output to an input, or
//! create a host [`OutputQueue`] from an output.

use std::marker::PhantomData;
use std::ptr::NonNull;

use depthai_sys as sys;

use crate::error::{check, take_native_error, take_string, Result};
use crate::message::Message;
use crate::node::NodeHandle;
use crate::queue::OutputQueue;

/// A node output, typed by the message it emits. Holds a reference to its node,
/// so it can never outlive the port it points at.
pub struct Output<M: Message> {
    node: NodeHandle,
    raw: NonNull<sys::dai_output>,
    _m: PhantomData<fn() -> M>,
}

impl<M: Message> Clone for Output<M> {
    fn clone(&self) -> Self {
        Output {
            node: self.node.clone(),
            raw: self.raw,
            _m: PhantomData,
        }
    }
}

// SAFETY: the raw pointer is owned by the node, which `node` keeps alive. Node
// ports have no locking of their own in depthai-core; every shim call on a port
// (name/link/unlink/create_queue) takes the shim's global graph mutex
// (DAI_LOCK_GRAPH in depthai_c.cpp), so concurrent use is serialised there.
unsafe impl<M: Message> Send for Output<M> {}
unsafe impl<M: Message> Sync for Output<M> {}

impl<M: Message> std::fmt::Debug for Output<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output")
            .field("name", &self.name().ok())
            .finish()
    }
}

impl<M: Message> Output<M> {
    /// # Safety
    /// `raw` must be a port owned by `node`.
    pub(crate) unsafe fn from_raw(node: NodeHandle, raw: NonNull<sys::dai_output>) -> Self {
        Output {
            node,
            raw,
            _m: PhantomData,
        }
    }

    fn raw(&self) -> *mut sys::dai_output {
        self.raw.as_ptr()
    }

    /// The port's descriptor name (e.g. `"depth"`).
    pub fn name(&self) -> Result<String> {
        let mut p = std::ptr::null_mut();
        check(unsafe { sys::dai_output_name(self.raw(), &mut p) })?;
        Ok(unsafe { take_string(p) })
    }

    /// Link this output into `input`. depthai rejects incompatible datatypes.
    pub fn link(&self, input: &Input) -> Result<()> {
        check(unsafe { sys::dai_output_link(self.raw(), input.raw()) })
    }

    pub fn unlink(&self, input: &Input) -> Result<()> {
        check(unsafe { sys::dai_output_unlink(self.raw(), input.raw()) })
    }

    /// Create a host queue fed by this output. `max_size` bounds it; `blocking`
    /// chooses whether a full queue stalls the producer (`true`) or drops the
    /// oldest message (`false`).
    pub fn create_output_queue(&self, max_size: u32, blocking: bool) -> Result<OutputQueue<M>> {
        let mut out: *mut sys::dai_queue = std::ptr::null_mut();
        check(unsafe {
            sys::dai_output_create_queue(self.raw(), max_size, blocking as i32, &mut out)
        })?;
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        // SAFETY: fresh owned handle.
        Ok(unsafe { OutputQueue::from_raw(raw) })
    }

    /// The node this port belongs to.
    pub fn node(&self) -> &NodeHandle {
        &self.node
    }

    /// Reinterpret as a different message type (no runtime check — the queue's
    /// `get` still verifies each message).
    pub fn cast<N: Message>(self) -> Output<N> {
        Output {
            node: self.node,
            raw: self.raw,
            _m: PhantomData,
        }
    }
}

/// A node input. Untyped: depthai checks datatype compatibility at link time.
#[derive(Clone)]
pub struct Input {
    node: NodeHandle,
    raw: NonNull<sys::dai_input>,
}

// SAFETY: as for Output — serialised by the shim's graph mutex.
unsafe impl Send for Input {}
unsafe impl Sync for Input {}

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Input")
    }
}

impl Input {
    /// # Safety
    /// `raw` must be a port owned by `node`.
    pub(crate) unsafe fn from_raw(node: NodeHandle, raw: NonNull<sys::dai_input>) -> Self {
        Input { node, raw }
    }

    pub(crate) fn raw(&self) -> *mut sys::dai_input {
        self.raw.as_ptr()
    }

    pub fn set_blocking(&self, blocking: bool) -> Result<()> {
        check(unsafe { sys::dai_input_set_blocking(self.raw(), blocking as i32) })
    }

    pub fn set_max_size(&self, max_size: u32) -> Result<()> {
        check(unsafe { sys::dai_input_set_max_size(self.raw(), max_size) })
    }

    pub fn node(&self) -> &NodeHandle {
        &self.node
    }

    /// Identity of the underlying port (two `Input`s from the same map key are equal).
    pub fn ptr_eq(&self, other: &Input) -> bool {
        self.raw == other.raw
    }
}
