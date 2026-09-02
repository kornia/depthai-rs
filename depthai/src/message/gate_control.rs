//! [`GateControl`]: the message that opens or closes a [`Gate`](crate::node::Gate).

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{out_handle, Result};
use crate::message::{typed_from_raw, AnyMessage, Message, Sealed};

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
        // -1 = the C++ default (unlimited / unthrottled).
        let arg = |o: Option<u32>| o.map_or(-1, |v| i32::try_from(v).unwrap_or(i32::MAX));
        let raw = out_handle(|out| unsafe {
            sys::dai_gate_control_new(open as i32, arg(num_messages), arg(fps), out)
        })?;
        // SAFETY: a fresh owned handle from the shim.
        unsafe { typed_from_raw(raw) }
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
