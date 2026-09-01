//! [`OutputQueue`]: the host side of a node output.

use std::marker::PhantomData;
use std::ptr::NonNull;
use std::time::Duration;

use depthai_sys as sys;

use crate::error::{check, check_poll, take_native_error, take_string, Result};
use crate::message::{Message, Msg};

/// A host queue created from an [`Output`](crate::Output) with
/// [`create_output_queue`](crate::Output::create_output_queue), typed by the
/// message it carries.
///
/// Methods take `&self`: depthai's `MessageQueue` is internally synchronised, so
/// a queue can be polled from one thread while another thread holds a clone of
/// the pipeline. Dropping the queue releases this handle only; the output stays
/// linked and the queue stays bounded by `max_size` (a *blocking* queue that
/// nobody drains will stall its producer — call [`close`](Self::close) first).
pub struct OutputQueue<M: Message> {
    raw: NonNull<sys::dai_queue>,
    _m: PhantomData<fn() -> M>,
}

// SAFETY: MessageQueue guards its state with a mutex + condvar; the handle is a
// shared_ptr copy.
unsafe impl<M: Message> Send for OutputQueue<M> {}
unsafe impl<M: Message> Sync for OutputQueue<M> {}

impl<M: Message> Drop for OutputQueue<M> {
    fn drop(&mut self) {
        unsafe { sys::dai_queue_release(self.raw.as_ptr()) };
    }
}

impl<M: Message> std::fmt::Debug for OutputQueue<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputQueue")
            .field("name", &self.name().ok())
            .finish()
    }
}

impl<M: Message> OutputQueue<M> {
    /// # Safety
    /// `raw` must be a live handle from `dai_output_create_queue`, owned by the caller.
    pub(crate) unsafe fn from_raw(raw: NonNull<sys::dai_queue>) -> Self {
        OutputQueue {
            raw,
            _m: PhantomData,
        }
    }

    fn raw(&self) -> *mut sys::dai_queue {
        self.raw.as_ptr()
    }

    fn wrap(&self, out: *mut sys::dai_msg) -> Result<M> {
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        // SAFETY: a fresh owned handle from the shim.
        unsafe { Msg::from_raw(raw) }.into_typed()
    }

    /// Pop the next message if one is queued (non-blocking).
    ///
    /// A message of the wrong type is consumed and reported as
    /// [`DepthaiError::UnexpectedDatatype`](crate::DepthaiError::UnexpectedDatatype).
    pub fn try_get(&self) -> Result<Option<M>> {
        let mut out: *mut sys::dai_msg = std::ptr::null_mut();
        if !check_poll(unsafe { sys::dai_queue_try_get(self.raw(), &mut out) })? {
            return Ok(None);
        }
        self.wrap(out).map(Some)
    }

    /// Block up to `timeout` for the next message. `Ok(None)` = timed out.
    pub fn get(&self, timeout: Duration) -> Result<Option<M>> {
        let ns = timeout.as_nanos().min(i64::MAX as u128) as i64;
        let mut out: *mut sys::dai_msg = std::ptr::null_mut();
        if !check_poll(unsafe { sys::dai_queue_get(self.raw(), ns, &mut out) })? {
            return Ok(None);
        }
        self.wrap(out).map(Some)
    }

    /// Block until the next message (errors if the queue is closed).
    pub fn get_blocking(&self) -> Result<M> {
        let mut out: *mut sys::dai_msg = std::ptr::null_mut();
        check_poll(unsafe { sys::dai_queue_get(self.raw(), -1, &mut out) })?;
        self.wrap(out)
    }

    pub fn has(&self) -> Result<bool> {
        let mut v = 0;
        check(unsafe { sys::dai_queue_has(self.raw(), &mut v) })?;
        Ok(v != 0)
    }

    pub fn len(&self) -> Result<u32> {
        let mut v = 0;
        check(unsafe { sys::dai_queue_size(self.raw(), &mut v) })?;
        Ok(v)
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Close the queue: pending `get`s return, producers stop being blocked by it.
    pub fn close(&self) -> Result<()> {
        check(unsafe { sys::dai_queue_close(self.raw()) })
    }

    pub fn is_closed(&self) -> Result<bool> {
        let mut v = 0;
        check(unsafe { sys::dai_queue_is_closed(self.raw(), &mut v) })?;
        Ok(v != 0)
    }

    pub fn set_blocking(&self, blocking: bool) -> Result<()> {
        check(unsafe { sys::dai_queue_set_blocking(self.raw(), blocking as i32) })
    }

    pub fn set_max_size(&self, max_size: u32) -> Result<()> {
        check(unsafe { sys::dai_queue_set_max_size(self.raw(), max_size) })
    }

    pub fn name(&self) -> Result<String> {
        let mut p = std::ptr::null_mut();
        check(unsafe { sys::dai_queue_name(self.raw(), &mut p) })?;
        Ok(unsafe { take_string(p) })
    }
}
