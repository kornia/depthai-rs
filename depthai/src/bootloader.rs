//! [`DeviceBootloader`]: the bootloader-side connection, used to reboot a wedged
//! device.

use std::ptr::NonNull;

use depthai_sys as sys;

use crate::device::DeviceInfo;
use crate::error::{out_handle, Result};

/// `dai::DeviceBootloader`. Constructing one connects to a device that is in the
/// bootloader; **dropping it reboots that device** back to an unbooted state so a
/// normal [`Device::open`](crate::Device::open) succeeds again. That open+drop is
/// the whole recovery for a PoE OAK stuck in `DeviceState::Bootloader`.
pub struct DeviceBootloader {
    raw: NonNull<sys::dai_bootloader>,
}

// SAFETY: opaque handle used only through the shim; not Sync (single owner).
unsafe impl Send for DeviceBootloader {}

impl std::fmt::Debug for DeviceBootloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceBootloader")
    }
}

impl DeviceBootloader {
    /// Connect to the bootloader of `info` (from [`Device::all_available`](crate::Device::all_available)).
    pub fn open(info: &DeviceInfo) -> Result<DeviceBootloader> {
        let raw_info = info.to_raw();
        let raw = out_handle(|out| unsafe { sys::dai_bootloader_open(&raw_info, out) })?;
        Ok(DeviceBootloader { raw })
    }
}

impl Drop for DeviceBootloader {
    fn drop(&mut self) {
        // SAFETY: created by dai_bootloader_open, released exactly once.
        unsafe { sys::dai_bootloader_release(self.raw.as_ptr()) };
    }
}
