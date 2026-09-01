//! [`CalibrationHandler`]: the factory calibration read from the device EEPROM.
//!
//! Nothing here applies defaults. depthai's own getters default the extrinsic
//! translation source and unit differently per method (`getCameraExtrinsics`
//! → calibrated/centimetres, `getBaselineDistance` → spec/centimetres), which is
//! an easy way to silently rescale a reconstruction, so every method that has
//! such a choice takes it as an explicit argument.

use std::ptr::NonNull;

use depthai_sys as sys;

use crate::enums::{CameraBoardSocket, CameraModel, LengthUnit};
use crate::error::{check, fill_vec, out_val, Result};

/// A snapshot of the device calibration (`dai::CalibrationHandler`).
#[derive(Debug)]
pub struct CalibrationHandler {
    raw: NonNull<sys::dai_calib>,
}

// SAFETY: the handler is an immutable value copy; every shim call is a const
// method on it.
unsafe impl Send for CalibrationHandler {}
unsafe impl Sync for CalibrationHandler {}

impl Drop for CalibrationHandler {
    fn drop(&mut self) {
        unsafe { sys::dai_calib_release(self.raw.as_ptr()) };
    }
}

impl CalibrationHandler {
    /// # Safety
    /// `raw` must be a live handle from `dai_device_read_calibration`, owned by the
    /// caller from now on.
    pub(crate) unsafe fn from_raw(raw: NonNull<sys::dai_calib>) -> Self {
        CalibrationHandler { raw }
    }

    fn raw(&self) -> *const sys::dai_calib {
        self.raw.as_ptr()
    }

    /// Intrinsics `[[fx, 0, cx], [0, fy, cy], [0, 0, 1]]` (row-major) of `socket`,
    /// scaled to `size` (`None` = the calibrated native resolution). A wiped EEPROM
    /// typically yields all zeros rather than an error — check `fx > 0`.
    pub fn camera_intrinsics(
        &self,
        socket: CameraBoardSocket,
        size: Option<(u32, u32)>,
    ) -> Result<[[f32; 3]; 3]> {
        let (w, h) = size.map_or((-1, -1), |(w, h)| (w as i32, h as i32));
        let mut k = [0f32; 9];
        check(unsafe {
            sys::dai_calib_camera_intrinsics(self.raw(), socket.to_raw(), w, h, k.as_mut_ptr())
        })?;
        Ok([[k[0], k[1], k[2]], [k[3], k[4], k[5]], [k[6], k[7], k[8]]])
    }

    /// Distortion coefficients in OpenCV order
    /// `[k1, k2, p1, p2, k3, k4, k5, k6, s1, s2, s3, s4, τx, τy]` — as many as the
    /// EEPROM carries (usually 14). Resolution-independent.
    pub fn distortion_coefficients(&self, socket: CameraBoardSocket) -> Result<Vec<f32>> {
        fill_vec::<f32>(32, |buf| {
            out_val(|n| unsafe {
                sys::dai_calib_distortion_coefficients(
                    self.raw(),
                    socket.to_raw(),
                    buf.as_mut_ptr(),
                    buf.len(),
                    n,
                )
            })
        })
    }

    pub fn distortion_model(&self, socket: CameraBoardSocket) -> Result<CameraModel> {
        out_val(|v| unsafe { sys::dai_calib_distortion_model(self.raw(), socket.to_raw(), v) })
            .map(CameraModel::from_raw)
    }

    /// The 4x4 transform `X_dst = T · X_src` (row-major).
    ///
    /// `use_spec_translation`: `true` takes the translation from the board design
    /// ("spec") data, `false` from the measured calibration. `unit`: the unit the
    /// translation column comes back in.
    pub fn camera_extrinsics(
        &self,
        src: CameraBoardSocket,
        dst: CameraBoardSocket,
        use_spec_translation: bool,
        unit: LengthUnit,
    ) -> Result<[[f32; 4]; 4]> {
        let mut t = [0f32; 16];
        check(unsafe {
            sys::dai_calib_camera_extrinsics(
                self.raw(),
                src.to_raw(),
                dst.to_raw(),
                use_spec_translation as i32,
                unit.to_raw(),
                t.as_mut_ptr(),
            )
        })?;
        Ok(to_4x4(&t))
    }

    /// IMU → camera transform (row-major 4x4). Errors when the EEPROM carries no
    /// IMU extrinsics. See [`camera_extrinsics`](Self::camera_extrinsics) for the
    /// arguments.
    pub fn imu_to_camera_extrinsics(
        &self,
        socket: CameraBoardSocket,
        use_spec_translation: bool,
        unit: LengthUnit,
    ) -> Result<[[f32; 4]; 4]> {
        let mut t = [0f32; 16];
        check(unsafe {
            sys::dai_calib_imu_to_camera_extrinsics(
                self.raw(),
                socket.to_raw(),
                use_spec_translation as i32,
                unit.to_raw(),
                t.as_mut_ptr(),
            )
        })?;
        Ok(to_4x4(&t))
    }

    /// Camera → IMU transform (row-major 4x4).
    pub fn camera_to_imu_extrinsics(
        &self,
        socket: CameraBoardSocket,
        use_spec_translation: bool,
        unit: LengthUnit,
    ) -> Result<[[f32; 4]; 4]> {
        let mut t = [0f32; 16];
        check(unsafe {
            sys::dai_calib_camera_to_imu_extrinsics(
                self.raw(),
                socket.to_raw(),
                use_spec_translation as i32,
                unit.to_raw(),
                t.as_mut_ptr(),
            )
        })?;
        Ok(to_4x4(&t))
    }

    /// `getBaselineDistance(cam1, cam2, ...)`. Prefer deriving the baseline from
    /// [`camera_extrinsics`](Self::camera_extrinsics) so it and the rotation come
    /// from one source.
    pub fn baseline_distance(
        &self,
        cam1: CameraBoardSocket,
        cam2: CameraBoardSocket,
        use_spec_translation: bool,
        unit: LengthUnit,
    ) -> Result<f32> {
        out_val(|v| unsafe {
            sys::dai_calib_baseline_distance(
                self.raw(),
                cam1.to_raw(),
                cam2.to_raw(),
                use_spec_translation as i32,
                unit.to_raw(),
                v,
            )
        })
    }

    pub fn stereo_left_socket(&self) -> Result<CameraBoardSocket> {
        out_val(|v| unsafe { sys::dai_calib_stereo_left_socket(self.raw(), v) })
            .map(CameraBoardSocket::from_raw)
    }

    pub fn stereo_right_socket(&self) -> Result<CameraBoardSocket> {
        out_val(|v| unsafe { sys::dai_calib_stereo_right_socket(self.raw(), v) })
            .map(CameraBoardSocket::from_raw)
    }

    /// Horizontal field of view in degrees. `use_spec`: from the board spec rather
    /// than the calibration.
    pub fn fov(&self, socket: CameraBoardSocket, use_spec: bool) -> Result<f32> {
        out_val(|v| unsafe { sys::dai_calib_fov(self.raw(), socket.to_raw(), use_spec as i32, v) })
    }
}

fn to_4x4(t: &[f32; 16]) -> [[f32; 4]; 4] {
    std::array::from_fn(|i| std::array::from_fn(|j| t[i * 4 + j]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn to_4x4_is_row_major() {
        let t: [f32; 16] = std::array::from_fn(|i| i as f32);
        let m = super::to_4x4(&t);
        assert_eq!(m[0][3], 3.0);
        assert_eq!(m[1][0], 4.0);
        assert_eq!(m[2][3], 11.0);
    }
}
