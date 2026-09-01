//! [`Camera`]: `dai::node::Camera`, the v3 unified camera node.

use std::ptr::NonNull;

use depthai_sys as sys;

use crate::enums::{CameraBoardSocket, ImgFrameType, ImgResizeMode};
use crate::error::{check, take_native_error, Result};
use crate::message::ImgFrame;
use crate::node::{node_type, NodeHandle};
use crate::port::Output;

/// A `dai::node::Camera`. Create it, [`build`](Self::build) it onto a socket,
/// then [`request_output`](Self::request_output) one or more streams.
#[derive(Clone)]
pub struct Camera(pub(crate) NodeHandle);
node_type!(Camera, dai_pipeline_create_camera);

impl Camera {
    /// `Camera::build(socket)`: bind to a board socket with automatic sensor
    /// resolution/fps selection.
    pub fn build(&self, socket: CameraBoardSocket) -> Result<&Self> {
        self.build_with(socket, None, None)
    }

    /// `Camera::build(socket, sensorResolution, sensorFps)`: bind with an explicit
    /// sensor mode.
    pub fn build_with(
        &self,
        socket: CameraBoardSocket,
        sensor_resolution: Option<(u32, u32)>,
        sensor_fps: Option<f32>,
    ) -> Result<&Self> {
        let (w, h) = sensor_resolution.map_or((-1, -1), |(w, h)| (w as i32, h as i32));
        check(unsafe {
            sys::dai_camera_build(
                self.0.raw(),
                socket.to_raw(),
                w,
                h,
                sensor_fps.unwrap_or(-1.0),
            )
        })?;
        Ok(self)
    }

    pub fn board_socket(&self) -> Result<CameraBoardSocket> {
        let mut v = 0;
        check(unsafe { sys::dai_camera_board_socket(self.0.raw(), &mut v) })?;
        Ok(CameraBoardSocket::from_raw(v))
    }

    /// `requestOutput(size, type, resizeMode, fps, enableUndistortion)`: a new
    /// output stream at `size`. `None` arguments take depthai's defaults.
    ///
    /// Note `undistort = Some(true)` undistorts but never *rectifies* (the
    /// rectifying rotation is identity); a stereo consumer that rectifies on the
    /// host should pass `Some(false)` and correct once, itself.
    pub fn request_output(
        &self,
        size: (u32, u32),
        img_type: Option<ImgFrameType>,
        resize_mode: ImgResizeMode,
        fps: Option<f32>,
        undistort: Option<bool>,
    ) -> Result<Output<ImgFrame>> {
        let mut out: *mut sys::dai_output = std::ptr::null_mut();
        check(unsafe {
            sys::dai_camera_request_output(
                self.0.raw(),
                size.0,
                size.1,
                img_type.map_or(-1, |t| t.to_raw()),
                resize_mode.to_raw(),
                fps.unwrap_or(-1.0),
                undistort.map_or(-1, |u| u as i32),
                &mut out,
            )
        })?;
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        // SAFETY: the port is owned by this node.
        Ok(unsafe { Output::from_raw(self.0.clone(), raw) })
    }

    /// `requestFullResolutionOutput(type, fps, useHighestResolution)`.
    pub fn request_full_resolution_output(
        &self,
        img_type: Option<ImgFrameType>,
        fps: Option<f32>,
        use_highest_resolution: bool,
    ) -> Result<Output<ImgFrame>> {
        let mut out: *mut sys::dai_output = std::ptr::null_mut();
        check(unsafe {
            sys::dai_camera_request_full_resolution_output(
                self.0.raw(),
                img_type.map_or(-1, |t| t.to_raw()),
                fps.unwrap_or(-1.0),
                use_highest_resolution as i32,
                &mut out,
            )
        })?;
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        Ok(unsafe { Output::from_raw(self.0.clone(), raw) })
    }
}
