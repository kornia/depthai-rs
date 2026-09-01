//! Error type and the FFI plumbing every wrapper call goes through.

use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;
use std::ptr::NonNull;

use depthai_sys as sys;

use crate::enums::Datatype;

/// Everything that can go wrong talking to depthai-core.
#[derive(Debug, thiserror::Error)]
pub enum DepthaiError {
    /// A C++ exception inside depthai-core (device lost, bad pipeline, XLink
    /// error, ...), with its message. This is the common case.
    #[error("depthai: {0}")]
    Native(String),
    /// A queue delivered a message of a different type than the queue was typed
    /// as. The message is consumed.
    #[error("unexpected message type: expected {expected:?}, got datatype {got}")]
    UnexpectedDatatype { expected: Datatype, got: i32 },
    /// A `&str` argument contained an interior NUL byte.
    #[error("argument contains a NUL byte: {0}")]
    Nul(#[from] std::ffi::NulError),
    /// The wrapper refused an argument before calling into depthai.
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
    /// depthai returned something the wrapper cannot represent (e.g. a
    /// calibration matrix that is not 3x3).
    #[error("malformed value from depthai: {0}")]
    Malformed(String),
}

pub type Result<T> = std::result::Result<T, DepthaiError>;

/// Read and clear the shim's thread-local error. Call on the SAME thread, right
/// after the failing FFI call (no await points in between).
pub(crate) fn take_native_error() -> DepthaiError {
    // SAFETY: dai_last_error never returns NULL and the string lives in a
    // thread-local until the next shim call on this thread.
    let msg = unsafe { CStr::from_ptr(sys::dai_last_error()) }
        .to_string_lossy()
        .into_owned();
    unsafe { sys::dai_clear_last_error() };
    DepthaiError::Native(msg)
}

/// Map a DAI_OK / DAI_ERR return code.
#[inline]
pub(crate) fn check(rc: c_int) -> Result<()> {
    if rc < 0 {
        Err(take_native_error())
    } else {
        Ok(())
    }
}

/// Map a 1 / 0 / -1 poll-style return code to `true` / `false` / `Err`.
#[inline]
pub(crate) fn check_poll(rc: c_int) -> Result<bool> {
    match rc {
        1 => Ok(true),
        0 => Ok(false),
        _ => Err(take_native_error()),
    }
}

/// Run a shim getter that returns its value through an out-parameter:
/// `out_val(|v| unsafe { sys::dai_queue_size(q, v) })?`. The FFI call stays
/// visible at the call site; only the temporary and the [`check`] move here.
#[inline]
pub(crate) fn out_val<T: Default>(call: impl FnOnce(&mut T) -> c_int) -> Result<T> {
    let mut v = T::default();
    check(call(&mut v))?;
    Ok(v)
}

/// [`out_val`] for the shim's `int`-as-bool out-parameters.
#[inline]
pub(crate) fn out_bool(call: impl FnOnce(&mut i32) -> c_int) -> Result<bool> {
    Ok(out_val(call)? != 0)
}

/// [`out_val`] for `char**` out-parameters: takes ownership of the string the
/// shim allocated and frees it.
pub(crate) fn out_string(call: impl FnOnce(&mut *mut c_char) -> c_int) -> Result<String> {
    let mut p: *mut c_char = std::ptr::null_mut();
    check(call(&mut p))?;
    if p.is_null() {
        return Ok(String::new());
    }
    // SAFETY: on success the shim wrote a live NUL-terminated string that we own.
    let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    unsafe { sys::dai_string_free(p) };
    Ok(s)
}

/// [`out_val`] for handle out-parameters. A call that succeeded but left the
/// handle NULL is reported as the shim's last error.
pub(crate) fn out_handle<T>(call: impl FnOnce(&mut *mut T) -> c_int) -> Result<NonNull<T>> {
    let mut raw: *mut T = std::ptr::null_mut();
    check(call(&mut raw))?;
    NonNull::new(raw).ok_or_else(take_native_error)
}

/// [`out_handle`] for poll-style calls: `Ok(None)` when the shim reports that
/// there is nothing to hand out.
pub(crate) fn poll_handle<T>(
    call: impl FnOnce(&mut *mut T) -> c_int,
) -> Result<Option<NonNull<T>>> {
    let mut raw: *mut T = std::ptr::null_mut();
    if !check_poll(call(&mut raw))? {
        return Ok(None);
    }
    NonNull::new(raw).ok_or_else(take_native_error).map(Some)
}

/// Read a static (non-owned) C string.
///
/// # Safety
/// `p` must be NULL or point at a NUL-terminated string with static lifetime.
pub(crate) unsafe fn static_string(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        // SAFETY: caller guarantees a NUL-terminated string with static lifetime.
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

pub(crate) fn cstring(s: &str) -> Result<CString> {
    Ok(CString::new(s)?)
}

/// Copy a NUL-terminated fixed-size C char array into a String.
pub(crate) fn fixed_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
