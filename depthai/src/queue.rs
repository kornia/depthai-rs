//! [`OutputQueue`]: the host side of a node output.

use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;
use std::time::Duration;

use depthai_sys as sys;

use crate::error::{
    check, duration_to_ns, out_bool, out_string, out_val, poll_handle, take_native_error, Result,
};
use crate::message::{typed_from_raw, Message};

#[derive(Debug)]
struct QueueInner {
    raw: NonNull<sys::dai_queue>,
}

// SAFETY: MessageQueue guards its state with a mutex + condvar; the handle is a
// shared_ptr copy.
unsafe impl Send for QueueInner {}
unsafe impl Sync for QueueInner {}

impl Drop for QueueInner {
    fn drop(&mut self) {
        unsafe { sys::dai_queue_release(self.raw.as_ptr()) };
    }
}

/// A host queue created from an [`Output`](crate::Output) with
/// [`create_output_queue`](crate::Output::create_output_queue), typed by the
/// message it carries.
///
/// Cloning shares the queue. Methods take `&self`: depthai's `MessageQueue` is
/// internally synchronised, so one thread can poll while another configures.
/// Dropping the last clone releases this handle only; the output stays linked and
/// the queue stays bounded by `max_size` (a *blocking* queue that nobody drains
/// will stall its producer — call [`close`](Self::close) first).
#[derive(Clone, Debug)]
pub struct OutputQueue<M: Message> {
    inner: Arc<QueueInner>,
    _m: PhantomData<fn() -> M>,
}

impl<M: Message> OutputQueue<M> {
    /// # Safety
    /// `raw` must be a live handle from `dai_output_create_queue`, owned by the caller.
    pub(crate) unsafe fn from_raw(raw: NonNull<sys::dai_queue>) -> Self {
        OutputQueue {
            inner: Arc::new(QueueInner { raw }),
            _m: PhantomData,
        }
    }

    fn raw(&self) -> *mut sys::dai_queue {
        self.inner.raw.as_ptr()
    }

    /// Pop the next message if one is queued (non-blocking).
    ///
    /// A message of the wrong type is consumed and reported as
    /// [`DepthaiError::UnexpectedDatatype`](crate::DepthaiError::UnexpectedDatatype).
    pub fn try_get(&self) -> Result<Option<M>> {
        let found = poll_handle(|out| unsafe { sys::dai_queue_try_get(self.raw(), out) })?;
        // SAFETY: a fresh owned handle from the shim.
        found.map(|raw| unsafe { typed_from_raw(raw) }).transpose()
    }

    /// Block up to `timeout` for the next message. `Ok(None)` = timed out.
    pub fn get(&self, timeout: Duration) -> Result<Option<M>> {
        let ns = duration_to_ns(timeout);
        let found = poll_handle(|out| unsafe { sys::dai_queue_get(self.raw(), ns, out) })?;
        // SAFETY: a fresh owned handle from the shim.
        found.map(|raw| unsafe { typed_from_raw(raw) }).transpose()
    }

    /// Block until the next message (errors if the queue is closed).
    pub fn get_blocking(&self) -> Result<M> {
        let found = poll_handle(|out| unsafe { sys::dai_queue_get(self.raw(), -1, out) })?;
        match found {
            // SAFETY: a fresh owned handle from the shim.
            Some(raw) => unsafe { typed_from_raw(raw) },
            None => Err(take_native_error()),
        }
    }

    pub fn has(&self) -> Result<bool> {
        out_bool(|v| unsafe { sys::dai_queue_has(self.raw(), v) })
    }

    pub fn len(&self) -> Result<u32> {
        out_val(|v| unsafe { sys::dai_queue_size(self.raw(), v) })
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Close the queue: pending `get`s return, producers stop being blocked by it.
    pub fn close(&self) -> Result<()> {
        check(unsafe { sys::dai_queue_close(self.raw()) })
    }

    pub fn is_closed(&self) -> Result<bool> {
        out_bool(|v| unsafe { sys::dai_queue_is_closed(self.raw(), v) })
    }

    pub fn set_blocking(&self, blocking: bool) -> Result<()> {
        check(unsafe { sys::dai_queue_set_blocking(self.raw(), blocking as i32) })
    }

    pub fn set_max_size(&self, max_size: u32) -> Result<()> {
        check(unsafe { sys::dai_queue_set_max_size(self.raw(), max_size) })
    }

    pub fn name(&self) -> Result<String> {
        out_string(|p| unsafe { sys::dai_queue_name(self.raw(), p) })
    }
}
