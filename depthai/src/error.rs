//! Error type and the FFI plumbing every wrapper call goes through.

use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;
use std::ptr::NonNull;
use std::time::Duration;

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
    #[error("unexpected message type: expected {expected:?}, got {got:?}")]
    UnexpectedDatatype { expected: Datatype, got: Datatype },
    /// A `&str` argument contained an interior NUL byte.
    #[error("argument contains a NUL byte: {0}")]
    Nul(#[from] std::ffi::NulError),
    /// A node type's fixed port is missing: the wrapper's port table and the
    /// linked depthai-core disagree.
    #[error("{node} has no {kind} named {name:?}")]
    MissingPort {
        node: String,
        kind: &'static str,
        name: String,
    },
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

/// [`out_string`] for the shim's newline-joined lists.
pub(crate) fn out_lines(call: impl FnOnce(&mut *mut c_char) -> c_int) -> Result<Vec<String>> {
    Ok(out_string(call)?
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect())
}

/// [`out_val`] for handle out-parameters. A call that succeeded but left the
/// handle NULL is reported as the shim's last error.
pub(crate) fn out_handle<T>(call: impl FnOnce(&mut *mut T) -> c_int) -> Result<NonNull<T>> {
    let mut raw: *mut T = std::ptr::null_mut();
    check(call(&mut raw))?;
    NonNull::new(raw).ok_or_else(take_native_error)
}

/// [`out_handle`] for poll-style (`1 / 0 / -1`) calls: `Ok(None)` when the shim
/// reports that there is nothing to hand out.
pub(crate) fn poll_handle<T>(
    call: impl FnOnce(&mut *mut T) -> c_int,
) -> Result<Option<NonNull<T>>> {
    let mut raw: *mut T = std::ptr::null_mut();
    match call(&mut raw) {
        1 => NonNull::new(raw).ok_or_else(take_native_error).map(Some),
        0 => Ok(None),
        _ => Err(take_native_error()),
    }
}

/// Run a shim "fill this buffer, report the total count" call, growing the
/// buffer once when the total exceeds `initial`.
pub(crate) fn fill_vec<T: Default + Clone>(
    initial: usize,
    mut fill: impl FnMut(&mut [T]) -> Result<usize>,
) -> Result<Vec<T>> {
    let mut buf = vec![T::default(); initial];
    let mut n = fill(&mut buf)?;
    if n > buf.len() {
        buf.resize(n, T::default());
        n = fill(&mut buf)?;
    }
    buf.truncate(n.min(buf.len()));
    Ok(buf)
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

/// A shim `int64_t` nanosecond count as a `Duration` (negative → zero).
pub(crate) fn duration_from_ns(ns: i64) -> Duration {
    Duration::from_nanos(ns.max(0) as u64)
}

/// A `Duration` as the shim's `int64_t` nanoseconds (saturating).
pub(crate) fn duration_to_ns(d: Duration) -> i64 {
    d.as_nanos().min(i64::MAX as u128) as i64
}

/// A `(ptr, len)` span the shim returned, as a slice; empty when NULL.
///
/// # Safety
/// The span must stay valid for `'a`.
pub(crate) unsafe fn bytes<'a>(p: *const u8, len: usize) -> &'a [u8] {
    if p.is_null() || len == 0 {
        &[]
    } else {
        // SAFETY: caller guarantees the span outlives 'a.
        unsafe { std::slice::from_raw_parts(p, len) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_conversions_saturate() {
        assert_eq!(duration_from_ns(-5), Duration::ZERO);
        assert_eq!(duration_from_ns(1_500), Duration::from_nanos(1_500));
        assert_eq!(duration_to_ns(Duration::from_secs(u64::MAX)), i64::MAX);
        assert_eq!(duration_to_ns(Duration::from_millis(2)), 2_000_000);
    }

    #[test]
    fn fill_vec_grows_once() {
        let mut calls = 0;
        let v = fill_vec::<u8>(2, |buf| {
            calls += 1;
            for (i, b) in buf.iter_mut().enumerate() {
                *b = i as u8;
            }
            Ok(5)
        })
        .unwrap();
        assert_eq!(v, vec![0, 1, 2, 3, 4]);
        assert_eq!(calls, 2);
        let w = fill_vec::<u8>(4, |_| Ok(1)).unwrap();
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn fixed_string_stops_at_nul() {
        assert_eq!(fixed_string(b"abc\0zzz"), "abc");
        assert_eq!(fixed_string(b"abc"), "abc");
    }
}
