//! [`MessageGroup`]: the [`Sync`](crate::node::Sync) node's output — several
//! messages grouped by name, time-aligned on the device.

use std::ptr::NonNull;
use std::time::Duration;

use depthai_sys as sys;

use crate::enums::Datatype;
use crate::error::{check, check_poll, cstring, take_native_error, take_string, Result};
use crate::message::{Message, Msg, Sealed};

/// A `dai::MessageGroup`.
#[derive(Clone, Debug)]
pub struct MessageGroup {
    msg: Msg,
}

impl Sealed for MessageGroup {}
impl Message for MessageGroup {
    const DATATYPE: Option<Datatype> = Some(Datatype::MessageGroup);

    unsafe fn from_msg(msg: Msg) -> Result<Self> {
        Ok(MessageGroup { msg })
    }

    fn as_msg(&self) -> &Msg {
        &self.msg
    }
}

impl MessageGroup {
    /// The member named `name` (the key it was linked into the Sync node's
    /// `inputs` map under), typed as `M`. `Ok(None)` when absent.
    pub fn get<M: Message>(&self, name: &str) -> Result<Option<M>> {
        let c = cstring(name)?;
        let mut out: *mut sys::dai_msg = std::ptr::null_mut();
        if !check_poll(unsafe { sys::dai_msg_group_get(self.msg.raw(), c.as_ptr(), &mut out) })? {
            return Ok(None);
        }
        let raw = NonNull::new(out).ok_or_else(take_native_error)?;
        // SAFETY: a fresh owned handle from the shim.
        let msg = unsafe { Msg::from_raw(raw) };
        msg.into_typed().map(Some)
    }

    /// Member names.
    pub fn names(&self) -> Result<Vec<String>> {
        let mut p = std::ptr::null_mut();
        check(unsafe { sys::dai_msg_group_names(self.msg.raw(), &mut p) })?;
        let s = unsafe { take_string(p) };
        Ok(s.lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect())
    }

    pub fn num_messages(&self) -> Result<i64> {
        let mut v = 0;
        check(unsafe { sys::dai_msg_group_num_messages(self.msg.raw(), &mut v) })?;
        Ok(v)
    }

    /// `isSynced(threshold)`: whether every member's timestamp lies within
    /// `threshold` of the others.
    pub fn is_synced(&self, threshold: Duration) -> Result<bool> {
        let mut v = 0;
        let ns = threshold.as_nanos().min(i64::MAX as u128) as i64;
        check(unsafe { sys::dai_msg_group_is_synced(self.msg.raw(), ns, &mut v) })?;
        Ok(v != 0)
    }

    /// `getIntervalNs()`: the spread between the earliest and latest member.
    pub fn interval(&self) -> Result<Duration> {
        let mut v = 0;
        check(unsafe { sys::dai_msg_group_interval_ns(self.msg.raw(), &mut v) })?;
        Ok(Duration::from_nanos(v.max(0) as u64))
    }
}
