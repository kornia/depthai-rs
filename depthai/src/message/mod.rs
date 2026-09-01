//! Messages that flow out of a pipeline: [`AnyMessage`] (a refcounted
//! `dai::ADatatype` handle carrying the [`MessageHeader`] every message has), the
//! [`Message`] trait that types a queue, and the typed messages.
//!
//! Every message type is `Clone` (a refcount bump) and `Send + Sync`: the payload
//! is immutable once delivered, so a frame can be handed to another thread, kept
//! past the next poll, or shared into a zero-copy image type, and its bytes stay
//! valid until the last clone drops.

mod encoded_frame;
mod img_frame;
mod imu_data;
mod message_group;

pub use encoded_frame::{EncodedFrame, EncodedFrameInfo};
pub use img_frame::{ImgFrame, ImgFrameInfo};
pub use imu_data::{ImuData, ImuPacket, ImuRotationVector, ImuVecReport, RawTimestamp};
pub use message_group::MessageGroup;

use std::ptr::NonNull;
use std::sync::Arc;
use std::time::Duration;

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{bytes, check, duration_from_ns, DepthaiError, Result};

/// The `dai::ADatatype` / `dai::Buffer` fields every message carries, read once
/// when the message is taken off its queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageHeader {
    pub datatype: Datatype,
    /// `getTimestamp()`: host `steady_clock` capture time, ns since that clock's
    /// epoch. Compare with [`steady_now`](crate::steady_now).
    pub timestamp_ns: i64,
    /// `getTimestampDevice()`: the device's own clock, ns since its boot.
    pub timestamp_device_ns: i64,
    pub sequence_num: i64,
    /// `getData().size()`.
    pub data_len: usize,
}

#[derive(Debug)]
struct MsgInner {
    raw: NonNull<sys::dai_msg>,
    header: MessageHeader,
    data: *const u8,
}

// SAFETY: the pointee is an immutable, atomically refcounted message; `data`
// points into the buffer it owns; every access goes through the shim's const
// accessors.
unsafe impl Send for MsgInner {}
unsafe impl Sync for MsgInner {}

impl Drop for MsgInner {
    fn drop(&mut self) {
        unsafe { sys::dai_msg_release(self.raw.as_ptr()) };
    }
}

/// A message of any type: a refcounted `dai::ADatatype` handle plus its
/// [`MessageHeader`]. The untyped form for queues whose type is not fixed;
/// downcast with [`downcast`](Self::downcast).
#[derive(Clone, Debug)]
pub struct AnyMessage {
    inner: Arc<MsgInner>,
}

impl AnyMessage {
    /// Take ownership of a handle the shim handed out and read its header.
    ///
    /// # Safety
    /// `raw` must be a live handle the caller owns (from a `dai_msg**` out-param).
    pub(crate) unsafe fn from_raw(raw: NonNull<sys::dai_msg>) -> Result<Self> {
        let mut info = sys::dai_buffer_info::default();
        // SAFETY: `raw` is live; `info` is a valid out-param.
        check(unsafe { sys::dai_buffer_get_info(raw.as_ptr(), &mut info) })?;
        let header = MessageHeader {
            datatype: Datatype::from_raw(info.datatype),
            timestamp_ns: info.timestamp_ns,
            timestamp_device_ns: info.timestamp_device_ns,
            sequence_num: info.sequence_num,
            data_len: info.data_len,
        };
        Ok(AnyMessage {
            inner: Arc::new(MsgInner {
                raw,
                header,
                data: info.data,
            }),
        })
    }

    pub(crate) fn raw(&self) -> *const sys::dai_msg {
        self.inner.raw.as_ptr()
    }

    pub fn header(&self) -> &MessageHeader {
        &self.inner.header
    }

    pub fn datatype(&self) -> Datatype {
        self.inner.header.datatype
    }

    /// The payload bytes (`dai::Buffer::getData()`), zero-copy, valid while any
    /// clone of this message lives.
    pub fn data(&self) -> &[u8] {
        // SAFETY: the span came from the shim for the buffer `inner` keeps alive.
        unsafe { bytes(self.inner.data, self.inner.header.data_len) }
    }

    /// View this message as `M`, checking the datatype.
    pub fn downcast<M: Message>(self) -> Result<M> {
        if let Some(expected) = M::DATATYPE {
            let got = self.datatype();
            if got != expected {
                return Err(DepthaiError::UnexpectedDatatype { expected, got });
            }
        }
        M::from_any(self)
    }
}

mod sealed {
    pub trait Sealed {}
}

/// A message type a queue or group can be typed as. Implemented by
/// [`ImgFrame`], [`EncodedFrame`], [`ImuData`], [`MessageGroup`] and
/// [`AnyMessage`]; sealed.
pub trait Message:
    sealed::Sealed + Clone + std::fmt::Debug + Send + Sync + Sized + 'static
{
    /// The datatype this type accepts (`None` = any).
    const DATATYPE: Option<Datatype>;

    /// Wrap a message whose datatype was verified against [`DATATYPE`](Self::DATATYPE).
    #[doc(hidden)]
    fn from_any(msg: AnyMessage) -> Result<Self>;

    /// The untyped message.
    fn as_any(&self) -> &AnyMessage;

    fn header(&self) -> &MessageHeader {
        self.as_any().header()
    }

    fn datatype(&self) -> Datatype {
        self.header().datatype
    }

    /// The payload bytes, zero-copy.
    fn data(&self) -> &[u8] {
        self.as_any().data()
    }

    /// Capture time on the host `steady_clock` (raw, ns).
    fn timestamp_ns(&self) -> i64 {
        self.header().timestamp_ns
    }

    /// Capture time on the host `steady_clock`.
    fn timestamp(&self) -> Duration {
        duration_from_ns(self.header().timestamp_ns)
    }

    /// Capture time on the device clock.
    fn timestamp_device(&self) -> Duration {
        duration_from_ns(self.header().timestamp_device_ns)
    }

    fn sequence_num(&self) -> i64 {
        self.header().sequence_num
    }
}

impl sealed::Sealed for AnyMessage {}
impl Message for AnyMessage {
    const DATATYPE: Option<Datatype> = None;
    fn from_any(msg: AnyMessage) -> Result<Self> {
        Ok(msg)
    }
    fn as_any(&self) -> &AnyMessage {
        self
    }
}

/// Wrap a fresh shim handle as `M`.
///
/// # Safety
/// `raw` must be a live handle the caller owns.
pub(crate) unsafe fn typed_from_raw<M: Message>(raw: NonNull<sys::dai_msg>) -> Result<M> {
    // SAFETY: forwarded from the caller.
    unsafe { AnyMessage::from_raw(raw) }?.downcast()
}

pub(crate) use sealed::Sealed;
