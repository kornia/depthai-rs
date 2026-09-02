//! [`NNArchive`]: a `dai::NNArchive` (model blob + config) loaded from disk.

use std::path::Path;
use std::ptr::NonNull;

use depthai_sys as sys;

use crate::error::{cstring, out_handle, Result};

/// A `dai::NNArchive`: the `.tar.xz` a [`get_model_from_zoo`](crate::get_model_from_zoo)
/// hands back, or one exported by the Luxonis tools. Immutable once loaded.
#[derive(Debug)]
pub struct NNArchive {
    raw: NonNull<sys::dai_nn_archive>,
}

// SAFETY: an NNArchive is read-only after construction (depthai copies it into
// the node on `build`/`setNNArchive`); the handle is a heap value we own.
unsafe impl Send for NNArchive {}
unsafe impl std::marker::Sync for NNArchive {}

impl Drop for NNArchive {
    fn drop(&mut self) {
        unsafe { sys::dai_nn_archive_release(self.raw.as_ptr()) };
    }
}

impl NNArchive {
    /// `NNArchive(path)`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = cstring(&path.as_ref().to_string_lossy())?;
        let raw = out_handle(|out| unsafe { sys::dai_nn_archive_open(path.as_ptr(), out) })?;
        Ok(NNArchive { raw })
    }

    /// `NNArchive::getInputSize(index)`: `(width, height)` of input `index`, `None`
    /// when the archive does not say.
    pub fn input_size(&self, index: u32) -> Result<Option<(u32, u32)>> {
        let (mut w, mut h) = (0u32, 0u32);
        match unsafe { sys::dai_nn_archive_input_size(self.raw.as_ptr(), index, &mut w, &mut h) } {
            1 => Ok(Some((w, h))),
            0 => Ok(None),
            _ => Err(crate::error::take_native_error()),
        }
    }

    pub(crate) fn raw(&self) -> *const sys::dai_nn_archive {
        self.raw.as_ptr()
    }
}
