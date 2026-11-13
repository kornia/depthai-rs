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
/// Immutable once built, like every message: a clone shares the one underlying message and
/// `send` queues it for another thread, so there are no setters. Build it with one of the
/// constructors.
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
    /// `CameraControl()` with `setManualFocus(lensPosition)`: `0..=255`; also turns autofocus off.
    pub fn manual_focus(lens_position: u8) -> Result<Self> {
        let raw = out_handle(|out| unsafe { sys::dai_camera_control_new(out) })?;
        // SAFETY: a fresh owned handle from the shim.
        let control: Self = unsafe { typed_from_raw(raw) }?;
        // SAFETY: `control` is the only handle to this message until it is returned, so
        // nothing else can read it while the shim sets the focus.
        check(unsafe {
            sys::dai_camera_control_set_manual_focus(control.any.raw().cast_mut(), lens_position)
        })?;
        Ok(control)
    }

    /// `CameraControl()` with `setAutoFocusMode(mode)`.
    pub fn auto_focus(mode: AutoFocusMode) -> Result<Self> {
        let raw = out_handle(|out| unsafe { sys::dai_camera_control_new(out) })?;
        // SAFETY: a fresh owned handle from the shim.
        let control: Self = unsafe { typed_from_raw(raw) }?;
        // SAFETY: as in `manual_focus`.
        check(unsafe {
            sys::dai_camera_control_set_auto_focus_mode(control.any.raw().cast_mut(), mode.to_raw())
        })?;
        Ok(control)
    }
}
