//! [`GateControl`]: the message that opens or closes a [`Gate`](crate::node::Gate).

use std::ptr::NonNull;

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{out_handle, Result};
use crate::message::{AnyMessage, Message, Sealed};

/// A `dai::GateControl`, built on the host and sent into a Gate's
/// `inputControl` through an [`InputQueue`](crate::InputQueue).
#[derive(Clone, Debug)]
pub struct GateControl {
    any: AnyMessage,
}

impl Sealed for GateControl {}
impl Message for GateControl {
    const DATATYPE: Option<Datatype> = Some(Datatype::GateControl);

    fn from_any(any: AnyMessage) -> Result<Self> {
        Ok(GateControl { any })
    }

    fn as_any(&self) -> &AnyMessage {
        &self.any
    }
}

impl GateControl {
    /// `GateControl(open, numMessages, fps)`: `num_messages` `None` = unlimited,
    /// `fps` `None` = unthrottled.
    pub fn new(open: bool, num_messages: Option<u32>, fps: Option<u32>) -> Result<Self> {
        let n = num_messages.map_or(-1, |v| v.min(i32::MAX as u32) as i32);
        let f = fps.map_or(-1, |v| v.min(i32::MAX as u32) as i32);
        let raw: NonNull<sys::dai_msg> =
            out_handle(|out| unsafe { sys::dai_gate_control_new(open as i32, n, f, out) })?;
        // SAFETY: a fresh owned handle from the shim.
        unsafe { AnyMessage::from_raw(raw) }?.downcast()
    }

    /// `GateControl::openGate()`: pass everything.
    pub fn open() -> Result<Self> {
        Self::new(true, None, None)
    }

    /// `GateControl::closeGate()`: drop everything.
    pub fn close() -> Result<Self> {
        Self::new(false, None, None)
    }

    /// `GateControl::openGate(numMessages, fps)`: pass the next `num_messages`
    /// (optionally throttled to `fps`), then close again.
    pub fn open_for(num_messages: u32, fps: Option<u32>) -> Result<Self> {
        Self::new(true, Some(num_messages), fps)
    }
}
