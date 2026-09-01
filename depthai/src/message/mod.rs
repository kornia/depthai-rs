//! Messages that flow out of a pipeline: the refcounted handle ([`Msg`]) every
//! typed message wraps, the [`Message`] trait that types a queue, and
//! [`AnyMessage`] for untyped queues.
//!
//! Every message type is `Clone` (a refcount bump on the underlying
//! `std::shared_ptr`) and `Send + Sync`: the payload is immutable once
//! delivered, so a frame can be handed to another thread, kept past the next
//! poll, or shared into a zero-copy image type, and its bytes stay valid until
//! the last clone drops.

mod encoded_frame;
mod img_frame;
mod imu_data;
mod message_group;

pub use encoded_frame::{EncodedFrame, EncodedFrameInfo};
pub use img_frame::{ImgFrame, ImgFrameInfo};
pub use imu_data::{ImuData, ImuPacket, ImuRotationVector, ImuVecReport, RawTimestamp};
pub use message_group::MessageGroup;

use std::ptr::NonNull;
use std::time::Duration;

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{check, take_native_error, DepthaiError, Result};

/// An owned reference to one depthai message (`std::shared_ptr<dai::ADatatype>`).
pub struct Msg {
    raw: NonNull<sys::dai_msg>,
}

// SAFETY: the pointee is an immutable, atomically refcounted message; every
// access goes through the shim's const accessors.
unsafe impl Send for Msg {}
unsafe impl Sync for Msg {}

impl Drop for Msg {
    fn drop(&mut self) {
        unsafe { sys::dai_msg_release(self.raw.as_ptr()) };
    }
}

impl Clone for Msg {
    fn clone(&self) -> Self {
        let mut out: *mut sys::dai_msg = std::ptr::null_mut();
        // SAFETY: `self.raw` is live; clone only fails on a null handle.
        let rc = unsafe { sys::dai_msg_clone(self.raw.as_ptr(), &mut out) };
        let raw = NonNull::new(out)
            .filter(|_| rc >= 0)
            .expect("dai_msg_clone on a live handle");
        Msg { raw }
    }
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Msg")
            .field("datatype", &self.datatype())
            .finish()
    }
}

impl Msg {
    /// # Safety
    /// `raw` must be a live handle the caller owns (from a `dai_msg**` out-param).
    pub(crate) unsafe fn from_raw(raw: NonNull<sys::dai_msg>) -> Self {
        Msg { raw }
    }

    pub(crate) fn raw(&self) -> *const sys::dai_msg {
        self.raw.as_ptr()
    }

    /// The message's runtime type.
    pub fn datatype(&self) -> Datatype {
        let mut v = 0;
        // A live handle cannot fail here; treat an error as "unknown".
        let rc = unsafe { sys::dai_msg_datatype(self.raw(), &mut v) };
        if rc < 0 {
            let _ = take_native_error();
            return Datatype::Other(-1);
        }
        Datatype::from_raw(v)
    }

    /// The payload bytes (`dai::Buffer::getData()`), valid while this handle or
    /// any clone of it lives.
    pub fn data(&self) -> Result<&[u8]> {
        let mut p: *const u8 = std::ptr::null();
        let mut len: usize = 0;
        check(unsafe { sys::dai_msg_data(self.raw(), &mut p, &mut len) })?;
        if p.is_null() || len == 0 {
            return Ok(&[]);
        }
        // SAFETY: the shim returned a span into the message's buffer, which lives
        // as long as `self`; the returned slice borrows `self`.
        Ok(unsafe { std::slice::from_raw_parts(p, len) })
    }

    /// `getTimestamp()`: host `steady_clock` time of capture, as nanoseconds since
    /// that clock's epoch. Compare with [`steady_now`](crate::steady_now).
    pub fn timestamp_ns(&self) -> Result<i64> {
        let mut v = 0;
        check(unsafe { sys::dai_msg_timestamp_ns(self.raw(), &mut v) })?;
        Ok(v)
    }

    /// `getTimestampDevice()`: the device's own clock, ns since its boot.
    pub fn timestamp_device_ns(&self) -> Result<i64> {
        let mut v = 0;
        check(unsafe { sys::dai_msg_timestamp_device_ns(self.raw(), &mut v) })?;
        Ok(v)
    }

    pub fn sequence_num(&self) -> Result<i64> {
        let mut v = 0;
        check(unsafe { sys::dai_msg_sequence_num(self.raw(), &mut v) })?;
        Ok(v)
    }

    /// Downcast into a typed message, checking the datatype.
    pub fn into_typed<M: Message>(self) -> Result<M> {
        if let Some(expected) = M::DATATYPE {
            let got = self.datatype();
            if got != expected {
                return Err(DepthaiError::UnexpectedDatatype {
                    expected,
                    got: got.to_raw(),
                });
            }
        }
        // SAFETY: datatype verified (or M accepts anything).
        unsafe { M::from_msg(self) }
    }
}

mod sealed {
    pub trait Sealed {}
}

/// A message type a queue or group can be typed as. Implemented by
/// [`ImgFrame`], [`EncodedFrame`], [`ImuData`], [`MessageGroup`] and
/// [`AnyMessage`]; sealed.
pub trait Message: sealed::Sealed + Send + Sync + Sized + 'static {
    /// The datatype this type accepts (`None` = any).
    const DATATYPE: Option<Datatype>;

    /// Wrap a raw message whose datatype has been verified.
    ///
    /// # Safety
    /// `msg.datatype()` must equal `Self::DATATYPE` (when `Some`).
    #[doc(hidden)]
    unsafe fn from_msg(msg: Msg) -> Result<Self>;

    /// The underlying handle.
    fn as_msg(&self) -> &Msg;

    /// Capture timestamp on the host `steady_clock` (see [`Msg::timestamp_ns`]).
    fn timestamp(&self) -> Duration {
        Duration::from_nanos(self.as_msg().timestamp_ns().unwrap_or(0).max(0) as u64)
    }

    /// Capture timestamp on the device clock (see [`Msg::timestamp_device_ns`]).
    fn timestamp_device(&self) -> Duration {
        Duration::from_nanos(self.as_msg().timestamp_device_ns().unwrap_or(0).max(0) as u64)
    }

    fn sequence_num(&self) -> i64 {
        self.as_msg().sequence_num().unwrap_or(-1)
    }
}

/// A message of any type, for queues whose type is not fixed. Downcast with
/// [`downcast`](Self::downcast).
#[derive(Clone, Debug)]
pub struct AnyMessage(pub(crate) Msg);

impl sealed::Sealed for AnyMessage {}
impl Message for AnyMessage {
    const DATATYPE: Option<Datatype> = None;
    unsafe fn from_msg(msg: Msg) -> Result<Self> {
        Ok(AnyMessage(msg))
    }
    fn as_msg(&self) -> &Msg {
        &self.0
    }
}

impl AnyMessage {
    pub fn datatype(&self) -> Datatype {
        self.0.datatype()
    }

    /// Try to view this message as `M`.
    pub fn downcast<M: Message>(self) -> Result<M> {
        self.0.into_typed()
    }

    /// The raw payload bytes.
    pub fn data(&self) -> Result<&[u8]> {
        self.0.data()
    }
}

pub(crate) use sealed::Sealed;
