//! [`Camera`]: `dai::node::Camera`, the v3 unified camera node.

use depthai_sys as sys;

use crate::enums::{opt_raw, CameraBoardSocket, ImgFrameType, ImgResizeMode};
use crate::error::{check, out_handle, out_val, Result};
use crate::message::ImgFrame;
use crate::node::node_type;
use crate::port::Output;

node_type!(
    /// A `dai::node::Camera`. Create it, [`build`](Self::build) it onto a socket,
    /// then [`request_output`](Self::request_output) one or more streams.
    Camera,
    dai_pipeline_create_camera
);

/// `Option<(w, h)>` as the shim's `-1, -1` = nullopt pair.
fn opt_size(size: Option<(u32, u32)>) -> (i32, i32) {
    size.map_or((-1, -1), |(w, h)| (w as i32, h as i32))
}

impl Camera {
    /// `Camera::build(socket)`: bind to a board socket with automatic sensor
    /// resolution/fps selection.
    pub fn build(&self, socket: CameraBoardSocket) -> Result<&Self> {
        self.build_with(socket, None, None)
    }

    /// `Camera::build(socket, sensorResolution, sensorFps)`: bind with an explicit
    /// sensor mode. A non-positive fps or a zero dimension selects the default
    /// (the shim's sentinels), rather than depthai's "invalid" error.
    pub fn build_with(
        &self,
        socket: CameraBoardSocket,
        sensor_resolution: Option<(u32, u32)>,
        sensor_fps: Option<f32>,
    ) -> Result<&Self> {
        let (w, h) = opt_size(sensor_resolution);
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
        out_val(|v| unsafe { sys::dai_camera_board_socket(self.0.raw(), v) })
            .map(CameraBoardSocket::from_raw)
    }

    /// `requestOutput(size, type, resizeMode, fps, enableUndistortion)`: a new
    /// output stream at `size`. `None` arguments take depthai's defaults, and so
    /// does a non-positive `fps` (the shim's sentinel is "<= 0").
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
        let raw = out_handle(|out| unsafe {
            sys::dai_camera_request_output(
                self.0.raw(),
                size.0,
                size.1,
                opt_raw(img_type),
                resize_mode.to_raw(),
                fps.unwrap_or(-1.0),
                undistort.map_or(-1, |u| u as i32),
                out,
            )
        })?;
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
        let raw = out_handle(|out| unsafe {
            sys::dai_camera_request_full_resolution_output(
                self.0.raw(),
                opt_raw(img_type),
                fps.unwrap_or(-1.0),
                use_highest_resolution as i32,
                out,
            )
        })?;
        // SAFETY: the port is owned by this node.
        Ok(unsafe { Output::from_raw(self.0.clone(), raw) })
    }
}
