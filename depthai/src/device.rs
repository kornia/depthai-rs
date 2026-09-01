//! [`Device`]: a connection to one OAK, and device enumeration.

use std::ptr::NonNull;
use std::sync::Arc;
use std::time::Duration;

use depthai_sys as sys;

use crate::calibration::CalibrationHandler;
use crate::enums::{CameraBoardSocket, DeviceState, Platform, UsbSpeed};
use crate::error::{
    check, cstring, fixed_string, out_bool, out_handle, out_string, out_val, Result,
};

pub(crate) struct DeviceInner {
    raw: NonNull<sys::dai_device>,
}

// SAFETY: the pointer is only dereferenced inside depthai-core, whose Device
// serialises every RPC behind its own mutex; the handle itself is a heap
// shared_ptr copy with an atomic control block.
unsafe impl Send for DeviceInner {}
unsafe impl Sync for DeviceInner {}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        // SAFETY: `raw` came from dai_device_open and is released exactly once.
        unsafe { sys::dai_device_release(self.raw.as_ptr()) };
    }
}

/// A connected OAK device. Cloning shares the same connection (like copying a
/// `std::shared_ptr<dai::Device>` in C++); the connection closes when the last
/// clone — and every [`Pipeline`](crate::Pipeline) built on it — is dropped.
#[derive(Clone)]
pub struct Device {
    inner: Arc<DeviceInner>,
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Device").finish_non_exhaustive()
    }
}

impl Device {
    /// Connect to a device.
    ///
    /// `id`: `None` = the first available device; `Some` = an MxId (USB or PoE),
    /// an IP address (PoE) or a device name. `max_usb_speed`: cap the USB link
    /// (`None` = depthai's default, USB 3). Ignored for PoE.
    pub fn open(id: Option<&str>, max_usb_speed: Option<UsbSpeed>) -> Result<Device> {
        let id_c = id.map(cstring).transpose()?;
        let id_ptr = id_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        let speed = max_usb_speed.map_or(-1, |s| s.to_raw());
        // SAFETY: id_ptr is NULL or a live C string.
        let raw = out_handle(|out| unsafe { sys::dai_device_open(id_ptr, speed, out) })?;
        Ok(Device {
            inner: Arc::new(DeviceInner { raw }),
        })
    }

    pub(crate) fn raw(&self) -> *mut sys::dai_device {
        self.inner.raw.as_ptr()
    }

    /// `dai::Device::getAllAvailableDevices()` — every OAK visible on USB and the
    /// network, with its state.
    pub fn all_available() -> Result<Vec<DeviceInfo>> {
        // Fills `buf` and reports the total device count, which may exceed it.
        let fill = |buf: &mut [sys::dai_device_info]| -> Result<usize> {
            // SAFETY: buf has `len` valid entries.
            out_val(|n| unsafe { sys::dai_device_all_available(buf.as_mut_ptr(), buf.len(), n) })
        };
        let mut buf = vec![sys::dai_device_info::default(); 16];
        let mut n = fill(&mut buf)?;
        if n > buf.len() {
            buf.resize(n, sys::dai_device_info::default());
            n = fill(&mut buf)?;
        }
        buf.truncate(n.min(buf.len()));
        Ok(buf.iter().map(DeviceInfo::from_raw).collect())
    }

    /// Close the connection now (also happens when the last clone drops).
    pub fn close(&self) -> Result<()> {
        check(unsafe { sys::dai_device_close(self.raw()) })
    }

    pub fn is_closed(&self) -> Result<bool> {
        out_bool(|v| unsafe { sys::dai_device_is_closed(self.raw(), v) })
    }

    /// The device's MxId.
    pub fn id(&self) -> Result<String> {
        out_string(|p| unsafe { sys::dai_device_id(self.raw(), p) })
    }

    /// The device's product name (e.g. `OAK-D-PRO`).
    pub fn name(&self) -> Result<String> {
        out_string(|p| unsafe { sys::dai_device_name(self.raw(), p) })
    }

    pub fn usb_speed(&self) -> Result<UsbSpeed> {
        Ok(UsbSpeed::from_raw(out_val(|v| unsafe {
            sys::dai_device_usb_speed(self.raw(), v)
        })?))
    }

    pub fn platform(&self) -> Result<Platform> {
        Ok(Platform::from_raw(out_val(|v| unsafe {
            sys::dai_device_platform(self.raw(), v)
        })?))
    }

    /// Which camera sockets are populated on this board.
    pub fn connected_cameras(&self) -> Result<Vec<CameraBoardSocket>> {
        let mut buf = [0i32; 16];
        let n = out_val(|n| unsafe {
            sys::dai_device_connected_cameras(self.raw(), buf.as_mut_ptr(), buf.len(), n)
        })?;
        Ok(buf[..n.min(buf.len())]
            .iter()
            .map(|&v| CameraBoardSocket::from_raw(v))
            .collect())
    }

    /// `getConnectedIMU()`: the raw firmware string naming the IMU chip. Empty or
    /// `"NONE"` on a board without one — that interpretation is the caller's.
    pub fn connected_imu(&self) -> Result<String> {
        out_string(|p| unsafe { sys::dai_device_connected_imu(self.raw(), p) })
    }

    /// Read the factory calibration from the EEPROM (one RPC; cache the result).
    pub fn read_calibration(&self) -> Result<CalibrationHandler> {
        let raw = out_handle(|out| unsafe { sys::dai_device_read_calibration(self.raw(), out) })?;
        // SAFETY: freshly created by the shim, owned by us from here on.
        Ok(unsafe { CalibrationHandler::from_raw(raw) })
    }

    /// Set the IR dot-projector intensity in `0.0..=1.0`. `mask`: `None` = all
    /// projectors. Returns depthai's success flag (`false` on boards without one).
    /// Needs a started pipeline on the device.
    pub fn set_ir_laser_dot_projector_intensity(
        &self,
        intensity: f32,
        mask: Option<i32>,
    ) -> Result<bool> {
        out_bool(|ok| unsafe {
            sys::dai_device_set_ir_laser_dot_projector_intensity(
                self.raw(),
                intensity,
                mask.unwrap_or(-1),
                ok,
            )
        })
    }

    /// Set the IR flood-light intensity in `0.0..=1.0`. See
    /// [`set_ir_laser_dot_projector_intensity`](Self::set_ir_laser_dot_projector_intensity).
    pub fn set_ir_flood_light_intensity(&self, intensity: f32, mask: Option<i32>) -> Result<bool> {
        out_bool(|ok| unsafe {
            sys::dai_device_set_ir_flood_light_intensity(
                self.raw(),
                intensity,
                mask.unwrap_or(-1),
                ok,
            )
        })
    }
}

/// One entry of [`Device::all_available`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// IP address (PoE), USB path, or name.
    pub name: String,
    /// MxId.
    pub device_id: String,
    pub state: DeviceState,
    /// `XLinkProtocol_t`, raw.
    pub protocol: i32,
    /// `XLinkPlatform_t`, raw.
    pub platform: i32,
    /// `XLinkError_t`, raw.
    pub status: i32,
}

impl DeviceInfo {
    pub(crate) fn from_raw(r: &sys::dai_device_info) -> Self {
        DeviceInfo {
            name: fixed_string(&r.name),
            device_id: fixed_string(&r.device_id),
            state: DeviceState::from_raw(r.state),
            protocol: r.protocol,
            platform: r.platform,
            status: r.status,
        }
    }

    pub(crate) fn to_raw(&self) -> sys::dai_device_info {
        let mut r = sys::dai_device_info::default();
        copy_fixed(&mut r.name, &self.name);
        copy_fixed(&mut r.device_id, &self.device_id);
        r.state = self.state.to_raw();
        r.protocol = self.protocol;
        r.platform = self.platform;
        r.status = self.status;
        r
    }
}

fn copy_fixed(dst: &mut [u8], s: &str) {
    let n = s.len().min(dst.len() - 1);
    dst[..n].copy_from_slice(&s.as_bytes()[..n]);
    dst[n] = 0;
}

/// `std::chrono::steady_clock::now()` as seen by depthai-core: the clock every
/// message timestamp is on. Use it to relate frame timestamps to wall time
/// (`SystemTime::now()` sampled at the same instant), which this crate
/// deliberately does not do for you.
pub fn steady_now() -> Result<Duration> {
    let ns = out_val(|ns| unsafe { sys::dai_steady_clock_now_ns(ns) })?;
    Ok(Duration::from_nanos(ns.max(0) as u64))
}

/// depthai-core's version string (e.g. `3.7.1`).
pub fn build_version() -> String {
    unsafe { crate::error::static_string(sys::dai_build_version()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_info_round_trips_through_pod() {
        let info = DeviceInfo {
            name: "192.168.1.42".into(),
            device_id: "18443010D1C3F00F00".into(),
            state: DeviceState::Bootloader,
            protocol: 2,
            platform: 1,
            status: 0,
        };
        let raw = info.to_raw();
        assert_eq!(DeviceInfo::from_raw(&raw), info);
    }

    #[test]
    fn device_info_truncates_long_names() {
        let info = DeviceInfo {
            name: "x".repeat(200),
            device_id: String::new(),
            state: DeviceState::Any,
            protocol: 0,
            platform: 0,
            status: 0,
        };
        let back = DeviceInfo::from_raw(&info.to_raw());
        assert_eq!(back.name.len(), 63);
    }
}
