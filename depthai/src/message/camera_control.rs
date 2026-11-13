//! [`CameraControl`]: the message that sets a camera's focus (and, later, its other controls).

use depthai_sys as sys;

use crate::enums::{AutoFocusMode, Datatype};
use crate::error::{check, out_handle, Result};
use crate::message::{typed_from_raw, AnyMessage, Message, Sealed};

/// A `dai::CameraControl`, built on the host and either installed with
/// [`Camera::set_initial_control`](crate::node::Camera::set_initial_control)
/// before the pipeline starts, or sent into the camera's
/// [`input_control`](crate::node::Camera::input_control) through an
/// [`InputQueue`](crate::InputQueue).
///
/// The setters take `&mut self` and mutate the underlying message in place:
/// finish them before you clone, share or send it (a clone shares the same
/// message).
#[derive(Clone, Debug)]
pub struct CameraControl {
    any: AnyMessage,
}

impl Sealed for CameraControl {}
impl Message for CameraControl {
    const DATATYPE: Option<Datatype> = Some(Datatype::CameraControl);

    fn from_any(any: AnyMessage) -> Result<Self> {
        Ok(CameraControl { any })
    }

    fn as_any(&self) -> &AnyMessage {
        &self.any
    }
}

impl CameraControl {
    /// `CameraControl()`: an empty control, which changes nothing until a setter runs.
    pub fn new() -> Result<Self> {
        let raw = out_handle(|out| unsafe { sys::dai_camera_control_new(out) })?;
        // SAFETY: a fresh owned handle from the shim.
        unsafe { typed_from_raw(raw) }
    }

    /// `setAutoFocusMode(mode)`.
    pub fn set_auto_focus_mode(&mut self, mode: AutoFocusMode) -> Result<&mut Self> {
        // SAFETY: `&mut self` is the only handle we hand out for mutation; the shim
        // mutates the message the handle owns.
        check(unsafe {
            sys::dai_camera_control_set_auto_focus_mode(self.any.raw().cast_mut(), mode.to_raw())
        })?;
        Ok(self)
    }

    /// `setManualFocus(lensPosition)`: `0..=255`; also turns autofocus off.
    pub fn set_manual_focus(&mut self, lens_position: u8) -> Result<&mut Self> {
        // SAFETY: as in `set_auto_focus_mode`.
        check(unsafe {
            sys::dai_camera_control_set_manual_focus(self.any.raw().cast_mut(), lens_position)
        })?;
        Ok(self)
    }
}
